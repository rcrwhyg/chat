use std::{convert::Infallible, time::Duration};

use axum::{
    // debug_handler,
    response::{sse::Event, Sse},
};
use axum_extra::{headers, TypedHeader};
use futures::stream::{self, Stream};
use tokio_stream::StreamExt as _;

// #[debug_handler]
pub(crate) async fn sse_handler(
    TypedHeader(user_agent): TypedHeader<headers::UserAgent>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    println!("`{}` connected", user_agent.as_str());

    let stream = stream::repeat_with(|| Event::default().data("hi!"))
        .map(Ok)
        .throttle(Duration::from_secs(1));

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(1))
            .text("keep-alive-text"),
    )
}
