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
use tower::{Layer, Service};
use tracing::Instrument;

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
}

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
            match data {
                EventData::AddPolicy(p) => guard.add_policy(p),
                EventData::AddGroupingPolicy(p) => guard.add_grouping_policy(p),
                EventData::AddPolicies(p) => guard.add_policies(p),
                EventData::AddGroupingPolicies(p) => guard.add_grouping_policies(p),
                EventData::RemovePolicy(p) => guard.remove_policy(p),
                EventData::RemoveGroupingPolicy(p) => guard.remove_grouping_policy(p),
                EventData::RemovePolicies(p) => guard.remove_policies(p),
                EventData::RemoveGroupingPolicies(p) => guard.remove_grouping_policies(p),
                EventData::RemoveFilteredPolicy(i, p) => guard.remove_filtered_policy(i, p),
                EventData::RemoveFilteredGroupingPolicy(i, p) => {
                    guard.remove_filtered_grouping_policy(i, p)
                }
            };
        }
    }
    .in_current_span();
    tokio::spawn(listener_loop);
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
