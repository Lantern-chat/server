use crate::prelude::*;

pub type ApiResult<T> = Result<T, Error>;

pub mod auth;
pub mod eps;

use ::rpc::{client::RpcClientError, procedure::Procedure, request::RpcRequest};
use schema::auth::RawAuthToken;

use ftl::{
    extract::real_ip::RealIpPrivacyMask,
    layers::rate_limit::{self, extensions::RateLimiterCallback},
    router::{HandlerService, Router},
    IntoResponse, Request, Response,
};

type Return = Result<Result<Procedure, Error>, ftl::Error>;

type InnerHandlerService = HandlerService<ServerState, Return>;
type RateLimitKey = (RealIpPrivacyMask,);

pub struct ApiV1Service {
    api: Router<ServerState, Return, rate_limit::RateLimitService<InnerHandlerService>>,
}

use self::auth::Auth;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReadyMarker;

impl ApiV1Service {
    pub async fn call(&self, req: Request) -> Result<Response, Error> {
        let (mut parts, body) = req.into_parts();

        let state = self.api.state();

        let mut auth_err: Option<Error> = None;

        let auth = match parts.headers.get(http::header::AUTHORIZATION) {
            Some(header) => match crate::auth::do_auth(state, &RawAuthToken::from_header(header.to_str()?)?).await
            {
                Ok(auth) => {
                    parts.extensions.insert(Auth(auth));
                    parts.extensions.insert(sdk::api::AuthMarker);

                    Some(auth)
                }
                Err(e) => {
                    /*
                        Avoid actually routing the request if the auth failed, but
                        fallback to the global rate-limiter and still invoke the router
                        to access the rate-limiter callback, then penalize later.
                    */
                    parts.uri = http::uri::Uri::from_static("/");
                    auth_err = Some(e);
                    None
                }
            },
            None => None,
        };

        if auth_err.is_none() {
            // use this to fail early if the user isn't authorized, since the absense
            // of this marker will cause the handlers to exit early.
            parts.extensions.insert(ReadyMarker);
        }

        let rl = RateLimiterCallback::<RateLimitKey>::default();
        parts.extensions.insert(rl.clone());

        let proc = match self.api.call_opt(Request::from_parts(parts, body)).await {
            // Rate-limit error is the only one allowed through directly as a response
            Err(rate_limit::Error::RateLimit(rate_limit_error)) => return Ok(rate_limit_error.into_response()),
            // if not found, signal a penalty to the rate-limiter as they should know better
            // and because it's not found, this will apply to their global rate-limit
            Ok(None) => Err(Error::NotFoundSignaling),

            Ok(Some(resp)) => match resp {
                Ok(proc) => proc,
                Err(e) => Err(e.into()),
            },
        };

        let Some(rl) = rl.get() else {
            return Err(Error::InternalErrorStatic("RateLimiterCallback not set"));
        };

        let try_proc = move || async move {
            if let Some(e) = auth_err {
                return Err(e);
            }

            let cmd = RpcRequest::Procedure {
                proc: proc?,
                addr: rl.key().0.into(), // hijack the rate-limiter key to get the IP address
                auth,
            };

            match state.rpc.send(&cmd).await {
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
            }
        };

        match try_proc().await {
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
        use sdk::api::{commands::all as cmds, Command, CommandFlags};

        let mut rl = rate_limit::RateLimitLayerBuilder::new();
        let mut api = Router::<_, Return>::with_state(state);

        macro_rules! add_cmds {
            ($($cmd:ty: $handler:expr),* $(,)?) => {$(
                GenericRouter::on(&mut api,
                    &[<$cmd as Command>::HTTP_METHOD],
                    <$cmd as Command>::ROUTE_PATTERN,
                    $handler,
                );

                rl.add_route(
                    (<$cmd as Command>::HTTP_METHOD, <$cmd as Command>::ROUTE_PATTERN), const {
                        let rl = <$cmd as Command>::RATE_LIMIT;
                        Quota::new(rl.emission_interval, rl.burst_size)
                    },
                );
            )*};

            // trivial handlers that just convert the extracted command to a procedure
            (@TRIVIAL $($cmd:ty),* $(,)?) => {$({
                add_cmds!($cmd: |_ready: ftl::extract::Extension<ReadyMarker>, auth: Option<Auth>, cmd: $cmd| {
                    // use generic ready future to avoid overhead from many near-duplicate async-block types
                    use core::future::ready;

                    // some routes don't need auth, some do, but that's checked in the command
                    if let Some(auth) = auth {
                        const FLAGS: CommandFlags = <$cmd as Command>::FLAGS;

                        if FLAGS.contains(CommandFlags::USERS_ONLY) && auth.is_bot() {
                            return ready(Err(Error::Unauthorized));
                        }

                        if FLAGS.contains(CommandFlags::BOTS_ONLY) && auth.is_user() {
                            return ready(Err(Error::Unauthorized));
                        }

                        if FLAGS.contains(CommandFlags::ADMIN_ONLY) && !auth.is_admin() {
                            return ready(Err(Error::Unauthorized));
                        }
                    }

                    ready(Ok(Procedure::from(cmd)))
                });
            })*};
        }

        add_cmds! { @TRIVIAL
            cmds::GetServerConfig,

            cmds::UserRegister,
            cmds::UserLogin,
            cmds::Enable2FA,
            cmds::Confirm2FA,
            cmds::Remove2FA,
            cmds::ChangePassword,
            cmds::GetSessions,
            cmds::ClearSessions,
            cmds::GetRelationships,
            cmds::PatchRelationship,
            cmds::UpdateUserProfile,
            cmds::GetUser,
            cmds::UpdateUserPrefs,
            cmds::UserLogout,

            cmds::CreateFile,
            cmds::GetFilesystemStatus,
            cmds::GetFileStatus,

            cmds::GetInvite,
            cmds::RevokeInvite,
            cmds::RedeemInvite,

            cmds::CreateParty,
            cmds::GetParty,
            cmds::PatchParty,
            cmds::DeleteParty,
            cmds::TransferOwnership,
            cmds::CreateRole,
            cmds::PatchRole,
            cmds::DeleteRole,
            cmds::GetPartyMembers,
            cmds::GetPartyMember,
            cmds::GetPartyRooms,
            cmds::GetPartyInvites,
            cmds::GetMemberProfile,
            cmds::UpdateMemberProfile,
            cmds::CreatePartyInvite,
            cmds::CreatePinFolder,
            cmds::CreateRoom,
            cmds::SearchParty,

            cmds::CreateMessage,
            cmds::EditMessage,
            cmds::GetMessage,
            cmds::DeleteMessage,
            cmds::StartTyping,
            cmds::GetMessages,
            cmds::PinMessage,
            cmds::UnpinMessage,
            cmds::StarMessage,
            cmds::UnstarMessage,
            cmds::PutReaction,
            cmds::DeleteOwnReaction,
            cmds::DeleteUserReaction,
            cmds::DeleteAllReactions,
            cmds::GetReactions,
            cmds::PatchRoom,
            cmds::DeleteRoom,
            cmds::GetRoom,
        }

        let default_quota = const {
            let rl = sdk::api::RateLimit::DEFAULT;
            Quota::new(rl.emission_interval, rl.burst_size)
        };

        Self {
            api: api.route_layer(
                rl.with_global_fallback(true).with_extension(true).with_default_quota(default_quota).build(),
            ),
        }
    }
}
