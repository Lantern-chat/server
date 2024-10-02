use std::time::Duration;

use crate::prelude::*;

pub type ApiResult<T> = Result<T, Error>;

use ::rpc::{client::RpcClientError, procedure::Procedure, request::RpcRequest};
use schema::auth::RawAuthToken;

use ftl::{
    extract::{real_ip::RealIpPrivacyMask, FromRequestParts},
    layers::{
        rate_limit::{
            extensions::RateLimiterCallback, Error as RateLimitError, RateLimitLayer, RateLimitLayerBuilder,
            RateLimitService,
        },
        resp_timing::StartTime,
    },
    router::{HandlerService, Router},
    IntoResponse, Request, RequestParts, Response,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Auth(pub Authorization);

impl core::ops::Deref for Auth {
    type Target = Authorization;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> FromRequestParts<S> for Auth {
    type Rejection = ftl::Error;

    fn from_request_parts(
        parts: &mut RequestParts,
        _: &S,
    ) -> impl core::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        core::future::ready(match parts.extensions.get::<Auth>() {
            Some(auth) => Ok(*auth),
            None => Err(ftl::Error::MissingHeader("Authorization")),
        })
    }
}

type Return = Result<Result<Procedure, Error>, ftl::Error>;

type InnerHandlerService = HandlerService<ServerState, Return>;
type RateLimitKey = (RealIpPrivacyMask,);

pub struct ApiV1Service {
    api: Router<ServerState, Return, RateLimitService<InnerHandlerService, RateLimitKey>>,
    rl: RateLimitLayer<RateLimitKey>,
}

impl ApiV1Service {
    pub async fn call(&self, req: Request) -> Result<Response, Error> {
        let state = self.api.state();

        let (mut parts, body) = req.into_parts();

        // get the start time for request. This is all guaranteed to be present.
        let StartTime(start) = *parts.extensions.get::<ftl::layers::resp_timing::StartTime>().unwrap();

        // extract rate-limiter key from the request before we start on real work
        let key = match RateLimitKey::from_request_parts(&mut parts, state).await {
            Ok(key) => key,
            Err(e) => return Err(e.into()),
        };

        parts.extensions.insert(key); // for reuse later by the per-route rate-limiter

        let global_rate_limiter = self.rl.global_fallback(key, None);

        // perform request within the global rate-limiter
        if let Err(e) = global_rate_limiter.req(start).await {
            return Ok(e.into_response());
        }

        // if an authorization token is present, perform the full authorization process regardless of the route
        let auth = match parts.headers.get(http::header::AUTHORIZATION) {
            None => None,
            Some(header) => {
                let raw_token = RawAuthToken::from_header(header.to_str()?)?;

                // wrap in async block for easier error handling
                let auth = async {
                    // check the cache first
                    match state.auth_cache.get(&raw_token) {
                        // fast path for cached tokens
                        Ok(Some(auth)) => Ok(auth),

                        // error from cache, such as an invalid token
                        Err(e) => Err(e),

                        // slow path for uncached tokens, fetch from RPC
                        Ok(None) => match state.rpc.authorize(raw_token).await {
                            Ok(Ok(auth)) => {
                                // insert the token into the cache for future requests
                                state.auth_cache.set(auth).await;

                                Ok(auth)
                            }
                            Ok(Err(e)) => {
                                // token is invalid or unauthorized, invalidate the cache and penalize the rate-limiter
                                if e.code == sdk::api::error::ApiErrorCode::Unauthorized {
                                    tokio::join! {
                                        // invalidate the token in the cache
                                        state.auth_cache.set_invalid(raw_token),

                                        // heavy penalty for invalid tokens
                                        global_rate_limiter.penalize(Duration::from_secs(1)),
                                    };
                                }

                                Err(Error::ApiError(e))
                            }
                            Err(rpc_error) => {
                                log::error!("Error authorizing token via RPC: {:?}", rpc_error);

                                Err(Error::InternalErrorStatic("RPC Error"))
                            }
                        },
                    }
                };

                let auth = auth.await?;

                parts.extensions.insert(Auth(auth));
                parts.extensions.insert(sdk::api::AuthMarker);

                Some(auth)
            }
        };

        // allow us to penalize the rate-limiter later if the request is not found or other errors occur
        let rlc = RateLimiterCallback::<RateLimitKey>::default();
        parts.extensions.insert(rlc.clone());

        // call the api router to get the procedure
        let proc = match self.api.call_opt(Request::from_parts(parts, body)).await {
            // Rate-limit error is the only one allowed through directly as a response
            Err(RateLimitError::RateLimit(rate_limit_error)) => return Ok(rate_limit_error.into_response()),
            // if the key is rejected, it failed to parse from the request
            // NOTE: Due to the above manual key extraction, this should be impossible
            Err(RateLimitError::KeyRejection(_)) => return Err(Error::BadRequest),
            // if not found, signal a penalty to the rate-limiter as they should know better
            // and because it's not found, this will apply to their global rate-limit
            Ok(None) => Err(Error::NotFoundSignaling),

            Ok(Some(resp)) => match resp {
                Ok(proc) => proc,
                Err(e) => Err(e.into()),
            },
        };

        // in potential rare error cases, rlc may not have been set, so fallback to the global rate-limiter
        let rl = rlc.get().unwrap_or(&global_rate_limiter);

        let try_proc = async move {
            // NOTE: This goes here because of proc?, it's just easier with the error handling below
            let cmd = RpcRequest::Procedure {
                proc: proc?,
                addr: rl.key().0.into(), // hijack the rate-limiter key to get the IP address
                auth: auth.map(Box::new),
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

        match try_proc.await {
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
        use ftl::layers::rate_limit::gcra::Quota;
        use ftl::router::GenericRouter;
        use sdk::api::{commands::all as cmds, Command, CommandFlags};

        let default_quota = const {
            let rl = sdk::api::RateLimit::DEFAULT;
            Quota::new(rl.emission_interval, rl.burst_size)
        };

        let mut rl = RateLimitLayerBuilder::new()
            .with_global_fallback(true)
            .with_extension(true)
            .with_default_quota(default_quota);

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
                add_cmds!($cmd: |auth: Option<Auth>, cmd: $cmd| {
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

        let rl = rl.build();

        Self {
            rl: rl.clone(),
            api: api.route_layer(rl),
        }
    }
}
