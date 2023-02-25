/// Distribute version of casbin role mapping layer.
/// It is used for further mapping rules with a distribute system.
/// Policies are protect by RwLock.
///
/// Initialize this layer with a [Stream] source(Output=[EventData]) additional
use crate::layer::role_mapping::enforce;
use casbin::{CoreApi, Event, EventEmitter, MgmtApi};
use futures::future::BoxFuture;
use futures::{Stream, StreamExt};
use http::{Request, Response};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;
use std::task::{Context, Poll};
use futures::executor::block_on;
use tower::{Layer, Service};
use tracing::{error, Instrument, warn};

#[derive(Clone)]
pub struct DistributeRoleMappingLayer<I, E> {
    enforcer: Arc<RwLock<E>>,
    _data: PhantomData<I>,
}

#[derive(Deserialize, Serialize)]
pub enum EventData {
    AddPolicy(Vec<String>),
    AddGroupingPolicy(Vec<String>),
    AddPolicies(Vec<Vec<String>>),
    AddGroupingPolicies(Vec<Vec<String>>),
    RemovePolicy(Vec<String>),
    RemoveGroupingPolicy(Vec<String>),
    RemovePolicies(Vec<Vec<String>>),
    RemoveGroupingPolicies(Vec<Vec<String>>),
    RemoveFilteredPolicy(usize, Vec<String>),
    RemoveFilteredGroupingPolicy(usize, Vec<String>),
    NIL,  // remain for failing deserializing event data
}

impl EventData {
    fn kind(&self) -> &'static str {
        match self {
            EventData::AddPolicy(_) => "AddPolicy",
            EventData::AddGroupingPolicy(_) => "AddGroupingPolicy",
            EventData::AddPolicies(_) => "AddPolicies",
            EventData::AddGroupingPolicies(_) => "AddGroupingPolicies",
            EventData::RemovePolicy(_) => "RemovePolicy",
            EventData::RemoveGroupingPolicy(_) => "RemoveGroupingPolicy",
            EventData::RemovePolicies(_) => "RemovePolicies",
            EventData::RemoveGroupingPolicies(_) => "RemoveGroupingPolicies",
            EventData::RemoveFilteredPolicy(_, _) => "RemoveFilteredPolicy",
            EventData::RemoveFilteredGroupingPolicy(_, _) => "RemoveFilteredGroupingPolicy",
            EventData::NIL => "NIL",
        }
    }
}

/// Safety:
///
/// Even rwlock guard is hold across await, the loop is only run in a specified thread called
/// `casbin event source loop`, that will not block the main thread in tokio or result in
/// deadlock.
///
/// Limit:
///
/// I enabled the `send_guard` feature in `parking_lock` for spawning thread, which is incompatible
/// with `deadlock_detection` feature. And `send_guard` feature might be insecure as it allow `!Send`
/// types like `Rc` protected by parking_lock and allow it's guard to be `Send`.
#[allow(clippy::await_holding_lock)]
fn listen_source<
    E: CoreApi + EventEmitter<Event> + Send + Sync + 'static,
    S: Stream<Item = EventData> + Send + 'static,
>(
    enforcer: Arc<RwLock<E>>,
    source: S,
) {
    let listener_loop = async move {
        tokio::pin!(source);
        while let Some(data) = source.next().await {
            let mut guard = enforcer.write();
            let kind = data.kind();
            let res = match data {
                EventData::AddPolicy(p) => guard.add_policy(p).await,
                EventData::AddGroupingPolicy(p) => guard.add_grouping_policy(p).await,
                EventData::AddPolicies(p) => guard.add_policies(p).await,
                EventData::AddGroupingPolicies(p) => guard.add_grouping_policies(p).await,
                EventData::RemovePolicy(p) => guard.remove_policy(p).await,
                EventData::RemoveGroupingPolicy(p) => guard.remove_grouping_policy(p).await,
                EventData::RemovePolicies(p) => guard.remove_policies(p).await,
                EventData::RemoveGroupingPolicies(p) => guard.remove_grouping_policies(p).await,
                EventData::RemoveFilteredPolicy(i, p) => guard.remove_filtered_policy(i, p).await,
                EventData::RemoveFilteredGroupingPolicy(i, p) => {
                    guard.remove_filtered_grouping_policy(i, p).await
                },
                _ => Ok(true),
            };
            match res {
                Ok(false) => warn!("Failed handle event data {:?}", kind),
                Err(e) => error!("Error handle event data, err: {}", e),
                _ => {}
            }
        }
    }
    .in_current_span();
    // spawn and detach the loop thread.
    std::thread::Builder::new()
        .name("casbin event source loop".to_string())
        .spawn(move || {
            block_on(listener_loop)
        }).expect("Cannot create event source loop thread.");
}

impl<I, E: CoreApi + EventEmitter<Event> + 'static> DistributeRoleMappingLayer<I, E> {
    /// source is where the policy changes comes from, it might be a message queue.
    pub fn new<S: Stream<Item = EventData> + Send + 'static>(enforcer: E, source: S) -> Self {
        let enforcer = Arc::new(RwLock::new(enforcer));
        listen_source(enforcer.clone(), source);
        Self {
            enforcer,
            _data: PhantomData::default(),
        }
    }
}

impl<S, I, E> Layer<S> for DistributeRoleMappingLayer<I, E> {
    type Service = DistributeRoleMapping<S, I, E>;

    fn layer(&self, inner: S) -> Self::Service {
        DistributeRoleMapping {
            inner,
            enforcer: self.enforcer.clone(),
            _data: PhantomData,
        }
    }
}

#[derive(Clone)]
pub struct DistributeRoleMapping<S, I, E> {
    inner: S,
    enforcer: Arc<RwLock<E>>,
    _data: PhantomData<I>,
}

impl<S, I, E, ReqBody, ResBody> Service<Request<ReqBody>> for DistributeRoleMapping<S, I, E>
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
        let guard = self.enforcer.read();
        let enforcer = guard.deref();
        enforce::<_, _, _, _, I>(&mut self.inner, req, enforcer)
    }
}
