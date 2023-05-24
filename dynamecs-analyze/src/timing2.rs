use std::time::Duration;
use crate::SpanPath;

struct AccumulatedSpanTimings {
    spans: Vec<SpanPath>,
    timings: Vec<Duration>,
}