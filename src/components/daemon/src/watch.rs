use crate::server::Server;
use request::{
    Aft, BlockOn, GetProject, Major, Pf, QueryMode, Rt, RtStIn, ShouldUnblock, TimeoutAt,
    TopErrors, UnwrapAft, log,
};
use smol::{Timer, future::FutureExt, lock::Mutex};
use std::{
    fmt::Debug,
    ops::DerefMut,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WatchConfig {
    pub interval_ms: u64,
    pub max_idle_time_ms: Option<u64>,
    pub cache_to_disk: bool,
}

impl WatchConfig {
    const fn new() -> Self {
        Self {
            interval_ms: 2000,
            max_idle_time_ms: None,
            cache_to_disk: true,
        }
    }
}

pub async fn watch<'e, P: Pf, REQ: Into<P::Req<'e>> + Clone + Send + UnwrapAft<'e, P>>(
    server: Arc<Server>,
    rt: Arc<Mutex<RtStIn<'e, P>>>,
    watchee: REQ,
) where
    P::Rev: Major,
    REQ::Aft<'e>: Debug,
    Aft<P>: From<<P as Pf>::Aft<'e>>,
    P::Req<'e>: From<GetProject>,
{
    let working_directory = Arc::<Path>::from(
        std::env::current_dir()
            .expect("failed to get working directory")
            .into_boxed_path(),
    );

    const DEFAULTS: WatchConfig = WatchConfig::new();
    let mut watch_config = DEFAULTS;

    loop {
        // NOTE: We keep the lock acquired for the entire lifetime of the query
        let mut rt = rt.lock().await;

        // Read the new project while we're at it.
        let new_project = watch_query(
            rt.deref_mut(),
            GetProject {
                working_directory: working_directory.clone(),
            },
            QueryMode::New,
            TimeoutAt(Instant::now() + Duration::from_millis(50)),
        )
        .await;

        dbg!(&new_project);

        // Determine if the watch config was changed as part of the project config.
        let new_watch_config = match new_project {
            Ok(BlockOn::Complete(Ok(project))) => {
                let config = WatchConfig {
                    interval_ms: project.interval_ms.unwrap_or(DEFAULTS.interval_ms),
                    cache_to_disk: project.cache_to_disk.unwrap_or(DEFAULTS.cache_to_disk),
                    max_idle_time_ms: project.max_idle_time_ms,
                };

                (config != watch_config).then_some(config)
            }
            Ok(BlockOn::Complete(Err(errors)))
                if !errors
                    .iter_unordered()
                    .any(|error| error.is_invalid_project_config_syntax()) =>
            {
                // Failed to parse project file, don't change the configuration
                None
            }
            _ => {
                // Failed to read project file
                (watch_config != DEFAULTS).then_some(DEFAULTS)
            }
        };

        // If we have a new watch config, update it as well as the idle tracker.
        if let Some(new_watch_config) = new_watch_config {
            watch_config = new_watch_config;

            server.idle_tracker.still_active();
            server
                .idle_tracker
                .set_max_idle_time(watch_config.max_idle_time_ms.map(Duration::from_millis));
            rt.cache_to_disk = watch_config.cache_to_disk;
        }

        let run = async {
            let timeout = TimeoutAt(Instant::now() + Duration::from_secs(2));

            Ok(watch_query(
                rt.deref_mut(),
                watchee.clone(),
                QueryMode::Continue,
                timeout,
            )
            .await)
        };

        if server.idle_tracker.shutdown_if_idle() {
            break;
        }

        let timeout = async {
            Timer::after(Duration::from_millis(500)).await;
            Err(())
        };

        log!("Watch Result is {:?}", run.or(timeout).await);
        Timer::after(Duration::from_millis(watch_config.interval_ms)).await;

        if server.idle_tracker.shutdown_if_idle() {
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
    query_mode: QueryMode,
    mut timeout: impl ShouldUnblock,
) -> Result<BlockOn<REQ::Aft<'e>, ()>, TopErrors>
where
    P::Aft<'e>: Into<Aft<P>>,
{
    let mut query = rt.query(req.into(), query_mode);

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
