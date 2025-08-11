use std::sync::{mpsc::Receiver, Arc, Mutex};

use crate::{Data, DeleteStatus};

pub fn notify_delete(data: &Arc<Mutex<Data>>, receiver: Receiver<DeleteStatus>) {
    let share_data = data.clone();
    rayon::spawn(move || {
        for status in receiver {
            let Ok(mut lock_data) = share_data.lock() else {
                continue;
            };

            let (is_deleted, index) = match status {
                DeleteStatus::Deleted(index) => (true, index),
                DeleteStatus::Error(index) => (false, index),
            };

            lock_data.update_delete_file(index, is_deleted);
            drop(lock_data);
        }
    });
}
