use std::{
    cmp::max,
    collections::{HashMap, HashSet},
    io::Write,
    thread::current,
};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Eval {
    callee: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Func {
    name: String,
    body: Option<String>,
    from_eval: Option<Eval>,
}

#[derive(Default)]
pub struct Repl {
    pub last_changed: Revision,
    pub root_file: File,
}

impl Repl {
    pub fn run(&mut self, current_revision: Revision) -> Option<Req> {
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
                    self.last_changed = current_revision;
                    for _ in self
                        .root_file
                        .funcs
                        .extract_if(.., |func| func.name == name)
                    {}
                }
                &["add", name] => {
                    self.last_changed = current_revision;
                    self.root_file.funcs.push(Func {
                        name: name.into(),
                        body: None,
                        from_eval: None,
                    });
                }
                &["add", name, body] => {
                    self.last_changed = current_revision;
                    self.root_file.funcs.push(Func {
                        name: name.into(),
                        body: Some(body.into()),
                        from_eval: None,
                    });
                }
                &["list"] => {
                    for func in self.root_file.funcs.iter() {
                        eprintln!("{:?}", func);
                    }
                    for eval in self.root_file.evals.iter() {
                        eprintln!("{:?}", eval);
                    }
                }
                &["addeval", name] => {
                    self.last_changed = current_revision;
                    self.root_file.evals.push(Eval {
                        callee: name.into(),
                    });
                }
                &["deleval", name] => {
                    self.last_changed = current_revision;
                    for _ in self
                        .root_file
                        .evals
                        .extract_if(.., |eval| eval.callee == name)
                    {}
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

impl Revision {
    pub fn next_iteration(&self) -> Self {
        Self {
            major: self.major,
            iteration: self.iteration + 1,
        }
    }
}

impl Ord for Revision {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.major
            .cmp(&other.major)
            .then(self.iteration.cmp(&other.iteration))
    }
}

impl PartialOrd for Revision {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
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
    Found(Vec<Func>),
    NewSymbolMap(SymbolMap),
    File(File),
    Error(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct File {
    funcs: Vec<Func>,
    evals: Vec<Eval>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Symbol(pub String);

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Req {
    MoveTowardsFixpoint,
    GetRootFile,
    GetSymbol(String),
    Pair(String, String),
}

impl Req {
    pub fn is_still_valid(
        &self,
        verified_at: Revision,
        symbol_map: &SymbolMap,
        repl: &Repl,
    ) -> bool {
        match self {
            Req::GetSymbol(name) => {
                if let Some(collection) = symbol_map.funcs.get(name) {
                    collection.last_changed <= verified_at
                } else {
                    true
                }
            }
            Req::GetRootFile => repl.last_changed <= verified_at,
            _ => false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FuncCollection {
    funcs: Vec<Func>,
    last_changed: Revision,
}

impl FuncCollection {
    pub fn push(&mut self, func: Func, revision: Revision) {
        self.funcs.push(func);
        self.last_changed = max(self.last_changed, revision);
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SymbolMap {
    pub funcs: HashMap<String, FuncCollection>,
    pub evals: HashSet<Eval>,
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
    let mut next_symbol_map = None;

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
                        && !req.is_still_valid(completed_task.task.verified_at, &symbol_map, &repl)
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
                    if let Artifact::NewSymbolMap(new_symbol_map) = &artifact {
                        if symbol_map != *new_symbol_map {
                            next_symbol_map = Some(new_symbol_map.clone());
                        }
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

        if next_symbol_map.is_none() || current_revision.iteration >= max_iterations {
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

            current_revision.major += 1;
            current_revision.iteration = 0;
            println!("REPL - REVISION {:?}", current_revision);

            let Some(new_req) = repl.run(current_revision) else {
                break;
            };

            start_req = Some(new_req.clone());
            queue.push(Req::MoveTowardsFixpoint);
            queue.push(new_req);
            continue;
        }

        eprintln!("next iteration");
        symbol_map = next_symbol_map.take().expect("new symbol map");
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
            let mut new_symbol_map = SymbolMap::default();

            let root_file = request(cache, symbol_map, repl, Req::GetRootFile, current_revision)?;
            let root_file = match root_file {
                Artifact::File(file) => file,
                _ => unreachable!("expected root file"),
            };

            for func in root_file.funcs.iter() {
                if let Some(existing) = symbol_map
                    .funcs
                    .get(&func.name)
                    .and_then(|funcs| funcs.funcs.iter().find(|f| *f == func))
                {
                    new_symbol_map
                        .funcs
                        .entry(func.name.clone())
                        .or_default()
                        .push(
                            existing.clone(),
                            symbol_map.funcs.get(&func.name).unwrap().last_changed,
                        );
                } else {
                    new_symbol_map
                        .funcs
                        .entry(func.name.clone())
                        .or_default()
                        .push(func.clone(), current_revision.next_iteration());
                }
            }

            for eval in root_file.evals.iter() {
                new_symbol_map.evals.insert(eval.clone());

                let callee = request(
                    cache,
                    symbol_map,
                    repl,
                    Req::GetSymbol(eval.callee.clone()),
                    current_revision,
                )?;

                match callee {
                    Artifact::Found(funcs) => match &funcs[..] {
                        [] => {
                            eprintln!("Function does not exist");
                        }
                        [f] => {
                            if let Some(body) = &f.body {
                                if let Some(existing) = symbol_map.funcs.get(body).and_then(|c| {
                                    c.funcs.iter().find(|f| f.from_eval.as_ref() == Some(eval))
                                }) {
                                    new_symbol_map.funcs.entry(body.into()).or_default().push(
                                        Func {
                                            name: body.into(),
                                            body: None,
                                            from_eval: Some(eval.clone()),
                                        },
                                        symbol_map.funcs.get(body).unwrap().last_changed,
                                    );
                                } else {
                                    new_symbol_map.funcs.entry(body.into()).or_default().push(
                                        Func {
                                            name: body.into(),
                                            body: None,
                                            from_eval: Some(eval.clone()),
                                        },
                                        current_revision.next_iteration(),
                                    );
                                }
                            }
                        }
                        [..] => {
                            eprintln!("Ambiguous function call");
                        }
                    },
                    _ => panic!("not callable"),
                }
            }

            // Make sure that any function deletions correctly update the last_changed
            // timestamp for the function collection.
            for (name, collection) in symbol_map.funcs.iter() {
                let Some(new_func_list) = new_symbol_map.funcs.get_mut(name) else {
                    // We have to keep a gravestone around for functions that used to exist, to
                    // show that anything that previously looked for them is outdated.
                    new_symbol_map.funcs.insert(
                        name.clone(),
                        FuncCollection {
                            funcs: vec![],
                            last_changed: if collection.funcs.is_empty() {
                                collection.last_changed
                            } else {
                                current_revision.next_iteration()
                            },
                        },
                    );
                    continue;
                };

                // If any function in the old symbol map is gone, then consider this a change
                for old_func in collection.funcs.iter() {
                    if !new_func_list.funcs.contains(old_func) {
                        new_func_list.last_changed = max(
                            new_func_list.last_changed,
                            current_revision.next_iteration(),
                        );
                        break;
                    }
                }
            }

            return Ok(Artifact::NewSymbolMap(new_symbol_map));
        }
        Req::GetSymbol(name) => {
            return Ok(Artifact::Found(
                symbol_map
                    .funcs
                    .get(name)
                    .into_iter()
                    .flat_map(|collection| collection.funcs.iter().cloned())
                    .collect(),
            ));
        }
        Req::GetRootFile => {
            return Ok(Artifact::File(repl.root_file.clone()));
        }
        Req::Pair(a, b) => {
            let a_artifact = request(
                cache,
                symbol_map,
                repl,
                Req::GetSymbol(a.into()),
                current_revision,
            )?;
            let b_artifact = request(
                cache,
                symbol_map,
                repl,
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
    repl: &Repl,
    req: Req,
    current_revision: Revision,
) -> Result<&'a Artifact, Vec<Req>> {
    eprintln!("Requesting {:?}", req);
    let Some(Some(TaskStatus::Completed(completed_task))) = cache.get(&req) else {
        eprintln!("  It's already running");
        return Err(vec![req]);
    };

    if completed_task.task.verified_at < current_revision
        && !req.is_still_valid(completed_task.task.verified_at, symbol_map, repl)
    {
        eprintln!("  It's out of date");
        return Err(vec![req]);
    }

    eprintln!("  It's verified for this revision");
    Ok(&completed_task.artifact)
}
