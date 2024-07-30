use crate::domain::channel_handle::Readonly;
use tokio::task;
use tokio::task::JoinHandle;

pub fn stop_task<T>(stop_handle: &Readonly<bool>, task: &JoinHandle<T>) -> JoinHandle<()> {
    let mut stop_receiver = stop_handle.get_receiver();
    let task_abort_handle = task.abort_handle();
    task::spawn(async move {
        if let Ok(stop) = stop_receiver.recv().await {
            if stop {
                task_abort_handle.abort();
            }
        }
    })
}
