use crate::{Major, Pf, RtStIn, TaskStatusKind, rt_st_in::query::RtStInQuery};

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
                    eprintln!("  Decrementing (running) {:?}", waiter);
                    running.left_waiting_on -= 1;

                    if running.left_waiting_on == 0 {
                        eprintln!("  Woke up (running) {:?}", waiter);
                        query.queue.push(waiter);
                    }
                }
                TaskStatusKind::Restarting(restarting) => {
                    eprintln!("  Decrementing (restarting) {:?}", waiter);
                    restarting.left_waiting_on -= 1;

                    if restarting.left_waiting_on == 0 {
                        eprintln!("  Woke up (restarting) {:?}", waiter);
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
