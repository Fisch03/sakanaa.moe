use super::types::*;
use anyhow::Result;
use axum::async_trait;
use core::fmt::Debug;
use std::sync::Arc;
use tokio::runtime::Handle;

mod fs_library_source;
mod musicbrainz_sources;
use fs_library_source::FsLibrarySource;
use musicbrainz_sources::{MusicBrainzLookupSource, MusicBrainzSearchSource};

#[async_trait]
pub trait AudioDataSource
where
    Self: Send + Sync,
{
    // these may be expensive operations and should therefore be run on a seperate thread.
    // returns Err if no new data was found (or if the lookup failed entirely)
    async fn lookup_track(
        &self,
        track: UnprocessedTrack,
        replace: bool,
    ) -> Result<UnprocessedTrack>;
}
impl Debug for dyn AudioDataSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AudioDataSource")
    }
}

#[derive(Clone, Debug)]
struct AudioLookupStep {
    priority: usize,
    replacing: bool,
    processor: Arc<dyn AudioDataSource>,
    next: Option<Vec<Arc<AudioLookupStep>>>,
}

pub struct AudioLookupPipeline {
    steps: Vec<Arc<AudioLookupStep>>,
}

impl AudioLookupPipeline {
    pub fn new() -> Self {
        // the lookup pipeline looks like this:
        //
        //               ┌─────┐
        //          ┌────┤Input├───┬────────┐
        //          │    └─────┘   │        │
        //     found│       no File│        │Title
        //      File│           but│        │only
        //          │          MBid│        │
        //          │              │        │
        //          ▼              ▼        ▼
        //       ┌────┐        ┌──────┐  ┌──────┐
        //     ┌─┤File├─┐      │ MBid │  │  MB  │
        //     │ └────┘ │no    │Lookup│  │Search│
        // MBid│        │MBid  └───┬──┘  └──┬───┘
        //     ▼        ▼          │        │
        // ┌──────-  ┌──────-      ├────────┤
        // │ MBid │  │  MB  │      │still   │
        // │Lookup│  │Search│      │no      │File
        // └───┬──┘  └──┬───┘      │File    ▼
        //     │        │          ▼     ┌────+
        //     └────────┴───┬────────────┤File│
        //                  │            └────┘
        //                  ▼
        //                ┌───┐
        //                │Out│
        //                └───┘
        //
        // + means override previous data
        // - means keep previous data

        let fs_library = Arc::new(FsLibrarySource::new());
        let mb_lookup = Arc::new(MusicBrainzLookupSource::new());
        let mb_search = Arc::new(MusicBrainzSearchSource::new());

        let mut steps = Vec::with_capacity(3);
        {
            // file first path (left side of the diagram)
            let mb_lookup_step = Arc::new(AudioLookupStep {
                priority: 0,
                processor: mb_lookup.clone(),
                replacing: false,
                next: None,
            });
            let mb_search_step = Arc::new(AudioLookupStep {
                priority: 1,
                processor: mb_search.clone(),
                replacing: false,
                next: None,
            });

            let file_step = Arc::new(AudioLookupStep {
                priority: 0,
                processor: fs_library.clone(),
                replacing: true,
                next: Some(vec![mb_lookup_step, mb_search_step]),
            });

            steps.push(file_step);
        }

        {
            // mbid lookup first path (middle path of the diagram)
            let file_step = Arc::new(AudioLookupStep {
                priority: 2,
                processor: fs_library.clone(),
                replacing: true,
                next: None,
            });

            let mb_lookup_step = Arc::new(AudioLookupStep {
                priority: 2,
                processor: mb_lookup.clone(),
                replacing: false,
                next: Some(vec![file_step.clone()]),
            });

            steps.push(mb_lookup_step);
        }

        {
            // mbid search first path (right side of the diagram)
            let file_step = Arc::new(AudioLookupStep {
                priority: 3,
                processor: fs_library.clone(),
                replacing: true,
                next: None,
            });

            let mb_search_step = Arc::new(AudioLookupStep {
                priority: 3,
                processor: mb_search.clone(),
                replacing: false,
                next: Some(vec![file_step]),
            });

            steps.push(mb_search_step);
        }

        Self { steps }
    }

    pub async fn lookup_track(&self, track: UnprocessedTrack) -> UnprocessedTrack {
        let found_track = {
            let mut track_tasks = Vec::new();
            let mut found_track = (None, usize::MAX);

            track_tasks.extend(self.steps.iter().map(|step| {
                let track = track.clone();
                let step = step.clone();
                let priority = step.priority;

                let task = tokio::task::spawn_blocking(move || {
                    Handle::current().block_on(async {
                        let processed_track = step
                            .processor
                            .lookup_track(track.clone(), step.replacing)
                            .await;

                        RunningStep {
                            track: processed_track.unwrap_or(track),
                            next: step.next.clone(),
                        }
                    })
                });

                (task, priority)
            }));

            while let Some(step) = track_tasks.pop() {
                let (running_step, priority) = step;

                let finished_step = running_step.await;
                if finished_step.is_err() {
                    continue;
                }
                let running_step = finished_step.unwrap();

                // abort early if everything is already filled in.
                // this *may* make results a bit less accurate but it potentially saves on
                // unnecessary api requests.
                if running_step.next.is_none() || running_step.track.is_ready() {
                    track_tasks = track_tasks
                        .into_iter()
                        .filter(|(task, prio)| {
                            if *prio >= priority {
                                println!("aborting task with prio {}", prio);
                                task.abort();
                                return false;
                            }
                            true
                        })
                        .collect();

                    if found_track.1 > priority {
                        found_track = (Some(running_step.track), priority);
                    }
                } else {
                    let next = running_step.next.as_ref().unwrap();
                    track_tasks.extend(next.iter().map(|next_step| {
                        let next_step = next_step.clone();
                        let track = running_step.track.clone();
                        let priority = next_step.priority;

                        let task = tokio::task::spawn_blocking(move || {
                            Handle::current().block_on(async {
                                let processed_track = next_step
                                    .processor
                                    .lookup_track(track.clone(), next_step.replacing)
                                    .await;

                                RunningStep {
                                    track: processed_track.unwrap_or(track),
                                    next: next_step.next.clone(),
                                }
                            })
                        });

                        (task, priority)
                    }));
                }
            }

            found_track.0
        };

        found_track.unwrap_or(track)
    }
}

#[derive(Debug)]
struct RunningStep {
    track: UnprocessedTrack,
    next: Option<Vec<Arc<AudioLookupStep>>>,
}
