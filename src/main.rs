extern crate google_gmail1 as gmail1;
pub mod cmd_args;
pub mod mail_service;

use clap::Parser;
use cmd_args::{self as cmd, Commands};
use lf_shardedringbuf::{spawn_buffer_task, LFShardedRingBuf, ShardPolicy};
use mail_service::{self as mail, get_msg_ids_from_messages};
use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};
use tokio::sync::Mutex as tokio_mutex;

// #[tokio::main]
// async fn main() {
fn main() {

    // use this BTreeSet as a Stream for enqueuers
    let msg_id_bts: Arc<tokio_mutex<BTreeSet<Option<String>>>> =
        Arc::new(tokio_mutex::new(BTreeSet::new()));
    let args = cmd::Args::parse();

    match args.cmds {
        Commands::Trash(trash) => {
            // building runtime with only the requested number of threads
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(trash.threads_num as usize)
                .enable_all()
                .build()
                .unwrap();

            // Create the sharded ring buffer and Gmail HttpConnector
            let msg_id_rb: Arc<LFShardedRingBuf<String>> =
                Arc::new(LFShardedRingBuf::new(1024, trash.threads_num as usize));
            
            let hub = runtime.block_on(async move {
                mail::create_client(&trash.secret_path, &trash.disk_token_path)
                    .await
                    .unwrap()
            });

            // Thread reference: https://doc.rust-lang.org/std/thread/
            let mut dequeuer_threads = Vec::with_capacity((trash.threads_num).try_into().unwrap());
            let mut enqueuer_threads = Vec::with_capacity((trash.threads_num).try_into().unwrap());

            let guard = runtime.enter();
            for i in 0..trash.threads_num {
                // Spawn trash enqueue tasks
                let msg_id_bts_clone = msg_id_bts.clone();
                let msg_id_rb_enqueue = Arc::clone(&msg_id_rb);
                let enqueue_thread = runtime.spawn(spawn_buffer_task(
                    ShardPolicy::ShiftBy {
                        initial_index: Some(i as usize),
                        shift: trash.threads_num as usize,
                    },
                    async move { mail::add_msgs(msg_id_bts_clone, &msg_id_rb_enqueue).await },
                ));
                enqueuer_threads.push(enqueue_thread);
                
                // Spawn trash dequeue tasks
                let hub_clone = hub.clone();
                let msg_id_rb_dequeue = Arc::clone(&msg_id_rb);
                let dequeue_thread = runtime.spawn(spawn_buffer_task(
                    ShardPolicy::ShiftBy {
                        initial_index: Some(i as usize),
                        shift: trash.threads_num as usize,
                    },
                    async move { mail::trash_msgs(&hub_clone, &msg_id_rb_dequeue).await },
                ));
                dequeuer_threads.push(dequeue_thread);
            }
            drop(guard);

            // Make sure to add in messages into the BTreeSet as a stream
            let msg_id_bts_clone = msg_id_bts.clone();
            runtime.block_on(async move {
                match trash.trash_opt {
                    cmd_args::TrashOptions::ByMsgIds(msg_ids) => {
                        mail::add_msg_ids_from_ids(&hub, msg_ids.msg_ids, msg_id_bts_clone).await;
                    }
                    cmd_args::TrashOptions::ByLabels(labels) => {
                        mail::add_msg_ids_from_labels(&hub, labels.labels, msg_id_bts_clone).await;
                    }
                    cmd_args::TrashOptions::ByFilter(filter) => {
                        mail::get_msg_ids_from_messages(
                            &hub,
                            None,
                            Some(&*filter),
                            msg_id_bts_clone,
                        )
                        .await;
                    }
                }
            });

            // Add in None values into the BTreeSet to signal to enqueuers that work is done
            let msg_id_bts_clone = msg_id_bts.clone();
            runtime.spawn(async move {
                for _ in 0..trash.threads_num {
                    let mut msg_id_bts_lock = msg_id_bts_clone.lock().await;
                    msg_id_bts_lock.insert(None);
                }
            });

            // Wait for enqueuers to finish first
            let messages_rec = runtime.block_on(async move {
                let mut messages = 0;
                while let Some(curr_thread) = enqueuer_threads.pop() {
                    messages += match curr_thread.await.unwrap() {
                        Ok(msg) => msg,
                        Err(_) => 0,
                    };
                }
                messages
            });

            // Then poison the ring buffer to let dequeuers terminate
            runtime.block_on(async { msg_id_rb.poison().await });

            // Then wait for dequeuers to finish
            let messages_tsh = runtime.block_on(async move {
                let mut messages = 0;
                while let Some(curr_thread) = dequeuer_threads.pop() {
                    messages += match curr_thread.await.unwrap() {
                        Ok(msg) => msg,
                        Err(_) => 0,
                    };
                }
                messages
            });

            assert_eq!(messages_tsh, messages_rec);
            println!("Trashed {} messages!", messages_tsh);
        }
        Commands::Send(send) => {
            let result = mail::send_message(*send.clone(), send.json_file);
            match result {
                Err(e) => {
                    println!("{:?}", e)
                }
                Ok(_res) => {}
            };
        }
        Commands::Labels(labels) => {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .build()
                .unwrap();
            let hub = runtime.block_on(async move {
                mail::create_client(&labels.secret_path, &labels.disk_token_path)
                    .await
                    .unwrap()
            });

            let labels_btreemap = runtime.block_on(async move { mail::list_labels(&hub).await });
            if let Ok(labels_btreemap) = labels_btreemap {
                let size = labels_btreemap.len();
                let mut count = 0;
                print!("All Labels in authenticated user's inbox: ");
                for label_id_pair in labels_btreemap {
                    count += 1;
                    if count != size {
                        print!("{}, ", label_id_pair.0);
                    } else {
                        print!("{}", label_id_pair.0);
                    }
                }
            }
        }
        Commands::Filter(filter) => {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(filter.threads as usize)
                .enable_all()
                .build()
                .unwrap();

            let msg_id_rb: Arc<LFShardedRingBuf<String>> =
                Arc::new(LFShardedRingBuf::new(1024, filter.threads as usize));

            let hub = runtime.block_on(async {
                mail::create_client(&filter.secret_path, &filter.disk_token_path)
                    .await
                    .unwrap()
            });
            let file_lock = Arc::new(Mutex::new(0));
            let mut dequeuer_threads = Vec::with_capacity((filter.threads).try_into().unwrap());
            let mut enqueuer_threads = Vec::with_capacity((filter.threads).try_into().unwrap());

            let guard = runtime.enter();
            for i in 0..filter.threads {
                // Spawn filter enqueue tasks
                let msg_id_bts_clone = msg_id_bts.clone();
                let msg_id_rb_enqueue = Arc::clone(&msg_id_rb);
                let enqueue_thread = runtime.spawn(spawn_buffer_task(
                    ShardPolicy::ShiftBy {
                        initial_index: Some(i as usize),
                        shift: filter.threads as usize,
                    },
                    async move { mail::add_msgs(msg_id_bts_clone, &msg_id_rb_enqueue).await },
                ));
                enqueuer_threads.push(enqueue_thread);
                
                // Spawn filter dequeue tasks
                let hub_clone = hub.clone();
                let output_file = filter.output.clone();
                let file_lock_clone = file_lock.clone();
                let msg_id_rb_dequeue = Arc::clone(&msg_id_rb);
                let dequeue_thread = runtime.spawn(spawn_buffer_task(
                    ShardPolicy::ShiftBy {
                        initial_index: Some(i as usize),
                        shift: filter.threads as usize,
                    },
                    async move {
                        mail::print_msgs(
                            &hub_clone,
                            &msg_id_rb_dequeue,
                            output_file,
                            file_lock_clone,
                        )
                        .await
                    },
                ));
                dequeuer_threads.push(dequeue_thread);
            }
            drop(guard);

            // Make sure to add in messages into the BTreeSet as a stream
            let msg_id_bts_clone = msg_id_bts.clone();
            runtime.block_on(async {
                get_msg_ids_from_messages(&hub, None, Some(&filter.filter), msg_id_bts_clone).await
            });

            // Add in None values into the BTreeSet to signal to enqueuers that work is done
            runtime.spawn(async move {
                for _ in 0..filter.threads {
                    // since last time using msg_id_bts here, no need to clone
                    let mut msg_id_bts_lock = msg_id_bts.lock().await;
                    msg_id_bts_lock.insert(None);
                }
            });

            // Wait for enqueuers to finish first
            let messages_found = runtime.block_on(async move {
                let mut messages = 0;
                while let Some(curr_thread) = enqueuer_threads.pop() {
                    messages += match curr_thread.await.unwrap() {
                        Ok(msg) => msg,
                        Err(_) => 0,
                    };
                }
                messages
            });

            // Then poison the ring buffer to let dequeuers terminate
            runtime.block_on(async { msg_id_rb.poison().await });

            // Then wait for dequeuers to finish
            let messages_printed = runtime.block_on(async move {
                let mut messages = 0;
                while let Some(curr_thread) = dequeuer_threads.pop() {
                    messages += match curr_thread.await.unwrap() {
                        Ok(msg) => msg,
                        Err(_) => 0,
                    };
                }
                messages
            });

            assert_eq!(messages_found, messages_printed);
            println!("Found {} messages!", messages_found);
        }
    }

    return;
}
