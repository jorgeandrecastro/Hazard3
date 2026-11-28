use hazard3_sync::blocking_mutex::raw::NoopRawMutex;
use hazard3_sync::pubsub::{PubSubChannel, SendDynSubscriber};

// NoopRawMutex is not Sync, so converting a Subscriber backed by it into a
// SendDynSubscriber must fail to compile.
fn main() {
    let channel = PubSubChannel::<NoopRawMutex, u32, 4, 4, 4>::new();
    let _sub: SendDynSubscriber<'_, u32> = channel.subscriber().unwrap().into();
}
