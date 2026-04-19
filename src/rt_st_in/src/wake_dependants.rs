use crate::{Major, Pf, RtStIn, RtStInQuery, TaskStatusKind, rt_trace};

pub fn wake_dependants<'e, P: Pf>(
    rt: &mut RtStIn<'e, P>,
    query: &mut RtStInQuery<'e, P>,
    req: &P::Req<'e>,
) where
    P::Rev: Major,
{
    if let Some(waiting) = query.waiting.remove(&req) {
        for waiter in waiting {
            match &mut rt
                .cache
                .get_mut(&waiter)
                .expect("waiter has cache entry")
                .as_mut()
                .expect("waiter is not processing")
                .kind
            {
                TaskStatusKind::Running(running) => {
                    rt_trace!("  Decrementing (running) {:?}", waiter);
                    running.left_waiting_on -= 1;

                    if running.left_waiting_on == 0 {
                        rt_trace!("  Woke up (running) {:?}", waiter);
                        query.queue.push(waiter);
                    }
                }
                TaskStatusKind::Restarting(restarting) => {
                    rt_trace!("  Decrementing (restarting) {:?}", waiter);
                    restarting.left_waiting_on -= 1;

                    if restarting.left_waiting_on == 0 {
                        rt_trace!("  Woke up (restarting) {:?}", waiter);
                        query.queue.push(waiter);
                    }
                }
                TaskStatusKind::Completed(_) => {
                    panic!("Expected waiter to be incomplete");
                }
            };
        }
    }
}
