use crate::server::Server;
use req::{Aft, BlockOn, Major, Pf, Rt, RtStIn, ShouldUnblock, TimeoutAt, TopErrors, UnwrapAft};
use smol::{Timer, future::FutureExt, lock::Mutex};
use std::{
    fmt::Debug,
    ops::DerefMut,
    sync::Arc,
    time::{Duration, Instant},
};

pub async fn watch<'e, P: Pf, REQ: Into<P::Req<'e>> + Clone + Send + UnwrapAft<'e, P>>(
    server: Arc<Server>,
    rt: Arc<Mutex<RtStIn<'e, P>>>,
    watchee: REQ,
) where
    P::Rev: Major,
    REQ::Aft<'e>: Debug,
    Aft<P>: From<<P as Pf>::Aft<'e>>,
{
    loop {
        // NOTE: We keep the lock acquired for the entire lifetime of the query
        let mut rt = rt.lock().await;

        let run = async {
            let timeout = TimeoutAt(Instant::now() + Duration::from_secs(2));
            Ok(watch_query(rt.deref_mut(), watchee.clone(), timeout).await)
        };

        let timeout = async {
            Timer::after(Duration::from_millis(500)).await;
            Err(())
        };

        let _ = dbg!(run.or(timeout).await);
        Timer::after(Duration::from_millis(2000)).await;

        if server.idle_tracker.lock().await.shutdown_if_idle() {
            break;
        }
    }
}

async fn watch_query<
    'a,
    'e,
    P: Pf,
    RT: Rt<'e, P>,
    REQ: Into<P::Req<'e>> + Send + UnwrapAft<'e, P>,
>(
    rt: &mut RT,
    req: REQ,
    mut timeout: impl ShouldUnblock,
) -> Result<BlockOn<REQ::Aft<'e>, ()>, TopErrors>
where
    P::Aft<'e>: Into<Aft<P>>,
{
    let mut query = rt.query(req.into());

    loop {
        // NOTE: Despite the processing by RT being synchronous,
        // we break it into small chucks so we can sort of fake it
        // being async by yielding to the executor often.
        let query_timeout = TimeoutAt(Instant::now() + Duration::from_millis(50));

        match rt.block_on(query, query_timeout)? {
            BlockOn::Complete(aft) => {
                return Ok(BlockOn::Complete(REQ::unwrap_aft(aft.into())));
            }
            BlockOn::Cyclic => return Ok(BlockOn::Cyclic),
            BlockOn::Diverges => return Ok(BlockOn::Diverges),
            BlockOn::TimedOut(new_query) => {
                if timeout.should_unblock() {
                    return Ok(BlockOn::TimedOut(()));
                }

                query = new_query;
            }
        }
    }
}
