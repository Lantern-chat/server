use crate::prelude::*;

pub type ApiResult<T> = Result<T, Error>;

pub mod auth;
pub mod eps;

use ::rpc::{client::RpcClientError, procedure::Procedure, request::RpcRequest};
use schema::auth::RawAuthToken;

use ftl::{
    extract::real_ip::RealIpPrivacyMask,
    layers::rate_limit,
    router::{HandlerService, Router},
    IntoResponse, Request, Response,
};

type Return = Result<Procedure, Response>;

type InnerHandlerService = HandlerService<ServerState, Return>;
type RateLimitKey = (RealIpPrivacyMask,);

pub struct ApiV1Service {
    api: Router<ServerState, Return, rate_limit::RateLimitService<InnerHandlerService>>,
}

use self::auth::Auth;

impl ApiV1Service {
    pub async fn call(&self, req: Request) -> Result<Response, Error> {
        let (mut parts, body) = req.into_parts();

        let state = self.api.state();

        let auth = match parts.headers.get(http::header::AUTHORIZATION) {
            Some(header) => {
                let auth = crate::auth::do_auth(state, &RawAuthToken::from_header(header.to_str()?)?).await?;

                parts.extensions.insert(Auth(auth));
                parts.extensions.insert(sdk::api::AuthMarker);

                Some(auth)
            }
            None => None,
        };

        let rl = rate_limit::extensions::RateLimiterCallback::<RateLimitKey>::default();

        parts.extensions.insert(rl.clone());

        // call the router and unpack all the possible results
        let proc = match self.api.call_opt(Request::from_parts(parts, body)).await {
            Ok(Some(resp)) => match resp {
                Ok(proc) => proc,
                Err(e) => return Ok(e), // Okay in the sense that it's a response
            },
            Ok(None) => return Err(Error::NotFound),
            Err(rate_limit::Error::RateLimit(rate_limit_error)) => return Ok(rate_limit_error.into_response()),
        };

        let Some(rl) = rl.get() else {
            return Err(Error::InternalErrorStatic("RateLimiterCallback not set"));
        };

        let cmd = RpcRequest::Procedure {
            proc,
            addr: rl.key().0.into(), // hijack the rate-limiter key to get the IP address
            auth,
        };

        let res = match state.rpc.send(&cmd).await {
            // penalize for non-existent resources
            Err(RpcClientError::DoesNotExist) => Err(Error::NotFoundHighPenalty),
            Err(e) => {
                log::error!("Error sending RPC request: {:?}", e);
                Err(Error::InternalErrorStatic("RPC Error"))
            }
            Ok(recv) => {
                let RpcRequest::Procedure { ref proc, .. } = cmd else {
                    unreachable!()
                };

                proc.stream_response::<_, Error>(recv).await
            }
        };

        match res {
            Ok(resp) => Ok(resp),
            Err(e) => {
                let penalty = e.penalty();

                if !penalty.is_zero() {
                    rl.penalize(penalty).await;
                }

                Err(e)
            }
        }
    }

    pub fn new(state: ServerState) -> Self {
        use ftl::router::GenericRouter;
        use rate_limit::gcra::Quota;
        use sdk::api::{commands::all as cmds, Command};

        let mut rl = rate_limit::RateLimitLayerBuilder::new();
        let mut api = Router::<_, Return>::with_state(state);

        macro_rules! add_cmds {
            ($($cmd:ty: $handler:expr),* $(,)?) => {
                $(
                    GenericRouter::on(&mut api,
                        &[<$cmd as Command>::HTTP_METHOD],
                        <$cmd as Command>::ROUTE_PATTERN,
                        $handler
                    );

                    rl.add_route(
                        (<$cmd as Command>::HTTP_METHOD, <$cmd as Command>::ROUTE_PATTERN),
                        {
                            let rl = <$cmd as Command>::RATE_LIMIT;

                            Quota::new(rl.emission_interval, rl.burst_size)
                        },
                    );
                )*
            };

            // trivial handlers that just convert the extracted command to a procedure
            (@TRIVIAL $($cmd:ty),* $(,)?) => {$({
                add_cmds!($cmd: |cmd: $cmd| core::future::ready(Procedure::from(cmd)));
            })*};
        }

        add_cmds! { @TRIVIAL
            cmds::UserRegister,
            cmds::UserLogin,
            cmds::UserLogout,
            cmds::Enable2FA,
            cmds::Confirm2FA,
            cmds::Remove2FA,
        }

        // add_cmds! {
        //     cmds::UserRegister: eps::register,
        // };

        // api.add("/auth", auth::auth);
        // api.add("/build", build::build);

        Self {
            api: api.route_layer(rl.build()),
        }
    }
}
