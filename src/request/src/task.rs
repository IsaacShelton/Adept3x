use crate::Pf;

#[derive(Clone, Debug)]
pub struct Task<'e, P: Pf> {
    pub verified_at: P::Rev,
    pub changed_at: P::Rev,
    pub requested: Vec<P::Req<'e>>,
}

#[derive(Debug)]
pub struct TaskStatus<'e, P: Pf> {
    pub kind: TaskStatusKind<'e, P>,
    pub task: Task<'e, P>,
}

#[derive(Debug)]
pub enum TaskStatusKind<'e, P: Pf> {
    Running(Running<'e, P>),
    Completed(Completed<'e, P>),
    Restarting(Restarting<'e, P>),
}

#[derive(Debug)]
pub struct Running<'e, P: Pf> {
    pub st: P::St<'e>,
    pub prev_aft: Option<P::Aft<'e>>,
    pub left_waiting_on: usize,
}

#[derive(Debug)]
pub struct Completed<'e, P: Pf> {
    pub aft: P::Aft<'e>,
}

#[derive(Debug)]
pub struct Restarting<'e, P: Pf> {
    pub prev_aft: P::Aft<'e>,
    pub left_waiting_on: usize,
    pub verified_at: P::Rev,
    pub deps_ready: bool,
}
