use std::{
    collections::{HashMap, HashSet},
    io::Write,
};

#[derive(Default)]
pub struct Repl {
    pub new_symbols: Vec<(String, Symbol)>,
    pub del_symbols: Vec<String>,
}

impl Repl {
    pub fn run(&mut self) -> Option<Req> {
        loop {
            let mut buffer = String::new();
            let stdin = std::io::stdin();

            print!("> ");
            let _ = std::io::stdout().flush();
            stdin.read_line(&mut buffer).unwrap();

            let parts: Vec<_> = buffer.split_whitespace().collect();

            match parts.as_slice() {
                &["get", name] => {
                    return Some(Req::GetSymbol(name.into()));
                }
                &["del", name] => {
                    self.del_symbols.push(name.into());
                }
                &["add", name, def] => {
                    self.new_symbols.push((name.into(), Symbol(def.into())));
                }
                &["pair", a, b] => {
                    return Some(Req::Pair(a.into(), b.into()));
                }
                &["quit" | "exit" | ".quit" | ".exit"] => {
                    return None;
                }
                _ => {
                    eprintln!("<invalid repl command>");
                }
            }
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Revision {
    major: usize,
    iteration: usize,
}

impl PartialOrd for Revision {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.major.partial_cmp(&other.major) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.iteration.partial_cmp(&other.iteration)
    }
}

#[derive(Clone, Debug)]
pub struct Task {
    pub verified_at: Revision,
    pub changed_at: Revision,
    pub requested: Vec<Req>,
}

#[derive(Debug)]
pub enum TaskState {
    Initial,
    Pair(Option<Artifact>, Option<Artifact>),
}

#[derive(Debug)]
pub struct RunningTask {
    pub state: TaskState,
    pub task: Task,
    pub prev_artifact: Option<Artifact>,
    pub left_waiting_on: usize,
}

#[derive(Debug)]
pub struct CompletedTask {
    pub artifact: Artifact,
    pub task: Task,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Artifact {
    Void,
    Found(Vec<Symbol>),
    AddSymbols(Vec<(String, Symbol)>),
    DelSymbols(Vec<String>),
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Symbol(pub String);

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Req {
    MoveTowardsFixpoint,
    GetSymbol(String),
    Pair(String, String),
}

impl Req {
    pub fn is_still_valid(&self, verified_at: Revision, symbol_map: &SymbolMap) -> bool {
        match self {
            Req::GetSymbol(name) => {
                if let Some(collection) = symbol_map.symbols.get(name) {
                    collection.last_changed <= verified_at
                } else {
                    true
                }
            }
            _ => false,
        }
    }
}

#[derive(Debug, Default)]
pub struct SymbolCollection {
    symbols: Vec<Symbol>,
    last_changed: Revision,
}

#[derive(Debug, Default)]
pub struct SymbolMap {
    pub symbols: HashMap<String, SymbolCollection>,
}

#[derive(Debug)]
pub enum TaskStatus {
    Running(RunningTask),
    Completed(CompletedTask),
}

type Cache = HashMap<Req, Option<TaskStatus>>;

pub fn main() {
    let mut repl = Repl::default();
    let mut symbol_map = SymbolMap::default();
    let mut queue = Vec::<Req>::new();
    let mut waiting = HashMap::<Req, Vec<Req>>::new();
    let mut cache = Cache::new();

    let mut start_req = None;
    let mut current_revision = Revision::default();
    current_revision.major += 1;

    loop {
        let mut progress = false;

        while let Some(req) = queue.pop() {
            // If the task has never been run before, start it
            let mut entry = cache.entry(req.clone()).or_insert_with(|| {
                Some(TaskStatus::Running(RunningTask {
                    state: TaskState::Initial,
                    task: Task {
                        verified_at: current_revision,
                        changed_at: current_revision,
                        requested: vec![],
                    },
                    prev_artifact: None,
                    left_waiting_on: 0,
                }))
            });

            // Acquire running task
            let mut running_task = match entry.take().expect("task to not be processing") {
                TaskStatus::Running(running_task) => running_task,
                TaskStatus::Completed(completed_task) => {
                    // If it hasn't been verified for this revision+iteration
                    if completed_task.task.verified_at < current_revision
                        && !req.is_still_valid(completed_task.task.verified_at, &symbol_map)
                    {
                        RunningTask {
                            state: TaskState::Initial,
                            task: Task {
                                verified_at: current_revision,
                                changed_at: completed_task.task.changed_at,
                                requested: vec![],
                            },
                            prev_artifact: Some(completed_task.artifact),
                            left_waiting_on: 0,
                        }
                    } else {
                        // Skip this task, as its result is already valid
                        let mut completed_task = completed_task;
                        completed_task.task.verified_at = current_revision;
                        *entry = Some(TaskStatus::Completed(completed_task));
                        continue;
                    }
                }
            };

            // Process the task
            let result = process(
                &cache,
                &req,
                &running_task.task,
                &mut running_task.state,
                &mut symbol_map,
                &mut repl,
            );

            let mut entry = cache.get_mut(&req).unwrap();
            eprintln!("Processing {:?}", req);

            // Check the result
            match result {
                Ok(artifact) => {
                    // If we added a symbol, we made progress towards the fixpoint,
                    // and the request to move towards the fixpoint should be re-requested.
                    if let Artifact::AddSymbols(symbols) = &artifact {
                        for (name, symbol) in symbols {
                            let collection = symbol_map.symbols.entry(name.clone()).or_default();
                            collection.last_changed = current_revision;
                            collection.last_changed.iteration += 1;
                            collection.symbols.push(symbol.clone());
                        }
                        progress = true;
                    }

                    // If we deleted a symbol, we made progress towards the fixpoint,
                    // and the request to move towards the fixpoint should be re-requested.
                    // Unlike adding symbols, this can happen via user input, hence
                    // the overall function is still monotonic.
                    if let Artifact::DelSymbols(symbols) = &artifact {
                        for name in symbols {
                            let collection = symbol_map.symbols.entry(name.clone()).or_default();
                            collection.last_changed = current_revision;
                            collection.symbols.clear();
                        }
                        progress = true;
                    }

                    // If the new completed artifact matches the old one
                    if Some(&artifact) == running_task.prev_artifact.as_ref() {
                        // Mark as complete, update the verified at, and don't
                        // queue any dependencies.
                        eprintln!("  The result for it hasn't changed");
                        *entry = Some(TaskStatus::Completed(CompletedTask {
                            artifact,
                            task: running_task.task,
                        }));
                    } else {
                        eprintln!("  It has a new result! {:?}", artifact);

                        // Mark as complete, updated the verified at and changed at.
                        // Re-queue any dependencies.
                        *entry = Some(TaskStatus::Completed(CompletedTask {
                            artifact: artifact,
                            task: running_task.task,
                        }));
                    }

                    if let Some(waiting) = waiting.remove(&req) {
                        for waiter in waiting {
                            let running_task = match cache
                                .get_mut(&waiter)
                                .expect("waiter has cache entry")
                                .as_mut()
                                .expect("waiter is not processing")
                            {
                                TaskStatus::Running(running_task) => running_task,
                                TaskStatus::Completed(completed_task) => {
                                    panic!("Expected waiter to be incomplete");
                                }
                            };

                            eprintln!("  Decrementing {:?}", waiter);
                            running_task.left_waiting_on -= 1;

                            if running_task.left_waiting_on == 0 {
                                eprintln!("  Woke up {:?}", waiter);
                                queue.push(waiter);
                            }
                        }
                    }
                }
                Err(dependencies) => {
                    eprintln!("  It has outdated dependencies");
                    running_task.left_waiting_on = 0;

                    for dep in dependencies {
                        match cache.get(&dep) {
                            Some(Some(TaskStatus::Completed(completed_task))) => {
                                if completed_task.task.verified_at >= current_revision {
                                    continue;
                                } else {
                                    // The completed result is stale, we need to requeue it
                                    queue.push(dep.clone());
                                    waiting.entry(dep).or_default().push(req.clone());
                                    running_task.left_waiting_on += 1;
                                }
                            }
                            Some(None | Some(TaskStatus::Running(_))) => {
                                // Already in queue/being processed
                                waiting.entry(dep).or_default().push(req.clone());
                                running_task.left_waiting_on += 1;
                            }
                            None => {
                                // Has never been invoked
                                queue.push(dep.clone());
                                waiting.entry(dep).or_default().push(req.clone());
                                running_task.left_waiting_on += 1;
                            }
                        }
                    }

                    // Re-queue immediately if everything requested is already ready and valid
                    if running_task.left_waiting_on == 0 {
                        queue.push(req.clone());
                    }

                    dbg!(&waiting);
                    let mut entry = cache.get_mut(&req).unwrap();
                    *entry = Some(TaskStatus::Running(running_task));
                }
            }
        }

        let max_iterations = 1000;

        if !progress || current_revision.iteration >= max_iterations {
            // Done evaluating, ask repl

            if current_revision.iteration >= max_iterations {
                println!(
                    "Fixpoint iteration limit exceeded! You're likely generating an infinite number of items."
                )
            } else if let Some(start_req) = &start_req {
                match cache
                    .get(&start_req)
                    .expect("cache entry to exist for started request")
                    .as_ref()
                    .expect("started request to not be processing anymore")
                {
                    TaskStatus::Running(_) => {
                        println!("Task is impossible, there is a cyclic dependency!")
                    }
                    TaskStatus::Completed(completed_task) => {
                        println!("{:?}", &completed_task.artifact)
                    }
                }
            }

            queue.clear();
            waiting.clear();

            let Some(new_req) = repl.run() else {
                break;
            };

            start_req = Some(new_req.clone());
            current_revision.major += 1;
            current_revision.iteration = 0;
            queue.push(Req::MoveTowardsFixpoint);
            queue.push(new_req);
            continue;
        }

        eprintln!("next iteration");
        current_revision.iteration += 1;
        queue.push(Req::MoveTowardsFixpoint);
        queue.push(start_req.clone().expect("starting request"));
    }
}

fn process(
    cache: &Cache,
    req: &Req,
    task: &Task,
    state: &mut TaskState,
    symbol_map: &SymbolMap,
    repl: &mut Repl,
) -> Result<Artifact, Vec<Req>> {
    let current_revision = task.verified_at;

    match req {
        Req::MoveTowardsFixpoint => {
            if !repl.new_symbols.is_empty() {
                return Ok(Artifact::AddSymbols(std::mem::take(&mut repl.new_symbols)));
            }
            if !repl.del_symbols.is_empty() {
                return Ok(Artifact::DelSymbols(std::mem::take(&mut repl.del_symbols)));
            } else {
                return Ok(Artifact::Void);
            }
        }
        Req::GetSymbol(name) => {
            return Ok(Artifact::Found(
                symbol_map
                    .symbols
                    .get(name)
                    .into_iter()
                    .flat_map(|collection| collection.symbols.iter().cloned())
                    .collect(),
            ));
        }
        Req::Pair(a, b) => {
            let a_artifact = request(
                cache,
                symbol_map,
                Req::GetSymbol(a.into()),
                current_revision,
            )?;
            let b_artifact = request(
                cache,
                symbol_map,
                Req::GetSymbol(b.into()),
                current_revision,
            )?;

            let mut result = vec![];
            match a_artifact {
                Artifact::Found(symbols) => result.extend(symbols.iter().cloned()),
                _ => (),
            }
            match b_artifact {
                Artifact::Found(symbols) => result.extend(symbols.iter().cloned()),
                _ => (),
            }
            Ok(Artifact::Found(result))
        }
    }
}

pub fn request<'a, 'b>(
    cache: &'a Cache,
    symbol_map: &'b SymbolMap,
    req: Req,
    current_revision: Revision,
) -> Result<&'a Artifact, Vec<Req>> {
    eprintln!("Requesting {:?}", req);
    let Some(Some(TaskStatus::Completed(completed_task))) = cache.get(&req) else {
        eprintln!("  It's already running");
        return Err(vec![req]);
    };

    if completed_task.task.verified_at < current_revision
        && !req.is_still_valid(completed_task.task.verified_at, symbol_map)
    {
        eprintln!("  It's out of date");
        return Err(vec![req]);
    }

    eprintln!("  It's verified for this revision");
    Ok(&completed_task.artifact)
}
