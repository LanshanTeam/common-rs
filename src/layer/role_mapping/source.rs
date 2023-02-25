use crate::layer::EventData;
use amqprs::channel::{BasicConsumeArguments, Channel, ConsumerMessage};
use futures::{ready, Stream, StreamExt};
use redis::Msg;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::warn;

pub async fn redis_source(
    channel: &str,
    conn: redis::aio::Connection,
) -> impl Stream<Item = EventData> + Send + 'static {
    let mut pub_sub = conn.into_pubsub();
    pub_sub
        .subscribe(channel)
        .await
        .unwrap_or_else(|_| panic!("Cannot subscribe channel {}", channel));
    let on_msg = pub_sub.into_on_message();
    on_msg.map(|msg: Msg| {
        let payload = msg.get_payload_bytes();
        serde_json::from_slice::<EventData>(payload).unwrap_or_else(|_| {
            warn!(
                "Cannot deserialize EventData({}) from redis",
                String::from_utf8_lossy(payload)
            );
            EventData::NIL
        })
    })
}

/// queue_name and a bind queue channel
pub async fn amqp_source(
    queue_name: &str,
    chan: Channel,
) -> impl Stream<Item = EventData> + Send + 'static {
    let (_, rx) = chan
        .basic_consume_rx(BasicConsumeArguments::new(
            queue_name,
            "role_mapping_event_data",
        ))
        .await
        .unwrap_or_else(|_| panic!("Cannot consume queue {}", queue_name));
    AMQPSource { rx }
}

pub struct AMQPSource {
    rx: UnboundedReceiver<ConsumerMessage>,
}

impl Stream for AMQPSource {
    type Item = EventData;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let msg = ready!(self.rx.poll_recv(cx));
        let data = msg.and_then(|msg| msg.content).map(|content| {
            serde_json::from_slice::<EventData>(content.as_slice()).unwrap_or_else(|_| {
                warn!(
                    "Cannot deserialize EventData({}) from rabbitmq",
                    String::from_utf8_lossy(content.as_slice())
                );
                EventData::NIL
            })
        });
        Poll::Ready(data)
    }
}

// todo other source...
