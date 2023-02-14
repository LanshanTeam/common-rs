/// Static casbin role mapping layer (policies will not change)
/// This is useful when use a RBAC role mapping without modify
/// policies, so it is lock-free.
///
/// This layer must be deploy behind [HttpAuthLayer] or [AsyncHttpAuthLayer]
/// which could insert the generic type I into request extensions.
///
/// Following are the object enforced with casbin:
/// obj => query path (/book, /user, etc)
/// act => http method (GET, POST, etc)
/// sub => request extension `I`  (uid, group, etc)
mod distribute;
mod source;

pub use distribute::*;
pub use source::*;

use casbin::CoreApi;
use futures::future::BoxFuture;
use http::{Request, Response, StatusCode};
use std::marker::PhantomData;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};
use tracing::warn;

#[derive(Clone)]
pub struct RoleMappingLayer<I, E> {
    enforcer: Arc<E>,
    _data: PhantomData<I>,
}

impl<I, E: CoreApi> RoleMappingLayer<I, E> {
    pub fn new(enforcer: E) -> Self {
        Self {
            enforcer: Arc::new(enforcer),
            _data: PhantomData::default(),
        }
    }
}

impl<S, I, E> Layer<S> for RoleMappingLayer<I, E> {
    type Service = RoleMapping<S, I, E>;

    fn layer(&self, inner: S) -> Self::Service {
        RoleMapping {
            inner,
            enforcer: self.enforcer.clone(),
            _data: PhantomData::default(),
        }
    }
}

#[derive(Clone)]
pub struct RoleMapping<S, I, E> {
    inner: S,
    enforcer: Arc<E>,
    _data: PhantomData<I>,
}

impl<S, I, E, ReqBody, ResBody> Service<Request<ReqBody>> for RoleMapping<S, I, E>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Send + 'static,
    S::Future: Send + 'static,
    ResBody: Default,
    I: AsRef<str> + Send + Sync + 'static,
    E: CoreApi,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        enforce::<_, _, _, _, I>(&mut self.inner, req, self.enforcer.as_ref())
    }
}

fn enforce<E: CoreApi, ReqBody, ResBody: Default, S, I>(
    inner: &mut S,
    req: Request<ReqBody>,
    enforcer: &E,
) -> BoxFuture<'static, Result<S::Response, S::Error>>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Send + 'static,
    S::Future: Send + 'static,
    I: AsRef<str> + Send + Sync + 'static,
{
    // obj => query path
    // act => http method
    // sub => request extension
    let sub = req
        .extensions()
        .get::<I>()
        .map(|sub| sub.as_ref())
        .unwrap_or("");
    let obj = req.uri().path();
    let act = req.method().as_str();

    match enforcer.enforce((sub, obj, act)) {
        Ok(checked) => {
            if checked {
                let fut = inner.call(req);
                Box::pin(async move { fut.await })
            } else {
                Box::pin(async move {
                    Ok(Response::builder()
                        .status(StatusCode::FORBIDDEN)
                        .body(ResBody::default())
                        .unwrap())
                })
            }
        }
        Err(err) => {
            warn!("enforcer is working abnormally, err: {:?}", err);
            Box::pin(async move {
                Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(ResBody::default())
                    .unwrap())
            })
        }
    }
}
