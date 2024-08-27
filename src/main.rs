extern crate google_gmail1 as gmail1;
pub mod cmd_args;
pub mod mail_service;
pub mod ringbuffer;

use clap::Parser;
use cmd_args::{self as cmd, Commands};
use mail_service::{self as mail, get_msg_ids_from_messages};
use ringbuffer::MultiThreadedRingBuffer;
use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};
use tokio::sync::Mutex as tokio_mutex;

#[tokio::main]
async fn main() {
    static MSG_ID_RB: MultiThreadedRingBuffer<String, 1024> = MultiThreadedRingBuffer::new();
    let msg_id_bts: Arc<tokio_mutex<BTreeSet<Option<String>>>> =
        Arc::new(tokio_mutex::new(BTreeSet::new()));
    let hub = mail::create_client().await.unwrap();
    let args = cmd::Args::parse();

    // println!("Args: {args:?}");

    match args.cmds {
        Commands::Trash(trash) => {
            // Thread reference: https://doc.rust-lang.org/std/thread/
            let mut dequerer_threads: Vec<tokio::task::JoinHandle<usize>> =
                Vec::with_capacity((trash.threads_num).try_into().unwrap());
            let mut enquerer_threads: Vec<tokio::task::JoinHandle<usize>> =
                Vec::with_capacity((trash.threads_num).try_into().unwrap());

            for _ in 0..trash.threads_num {
                let hub_clone = hub.clone();
                let msg_id_bts_clone = msg_id_bts.clone();
                let dequeue_thread =
                    tokio::spawn(async move { mail::trash_msgs(&hub_clone, &MSG_ID_RB).await });
                let enqueue_thread =
                    tokio::spawn(async move { mail::add_msgs(msg_id_bts_clone, &MSG_ID_RB).await });
                dequerer_threads.push(dequeue_thread);
                enquerer_threads.push(enqueue_thread);
            }

            match trash.trash_opt {
                cmd_args::TrashOptions::ByMsgIds(msg_ids) => {
                    mail::add_msg_ids_from_ids(&hub, msg_ids.msg_ids, msg_id_bts.clone()).await;
                }
                cmd_args::TrashOptions::ByLabels(labels) => {
                    mail::add_msg_ids_from_labels(&hub, labels.labels, msg_id_bts.clone()).await;
                }
                cmd_args::TrashOptions::ByFilter(filter) => {
                    mail::get_msg_ids_from_messages(&hub, None, Some(*filter), msg_id_bts.clone())
                        .await;
                }
            }

            for _ in 0..trash.threads_num {
                let mut msg_id_bts_lock = msg_id_bts.lock().await;
                msg_id_bts_lock.insert(None);
            }

            MSG_ID_RB.poison().await;

            let mut messages_trashed: usize = 0;
            let mut messages_received: usize = 0;
            while let Some(curr_thread) = dequerer_threads.pop() {
                messages_trashed += curr_thread.await.unwrap();
            }

            while let Some(curr_thread) = enquerer_threads.pop() {
                messages_received += curr_thread.await.unwrap();
            }

            assert_eq!(messages_trashed, messages_received);
            println!("Trashed {} messages!", messages_trashed);
        }
        Commands::Send(send) => {
            let result = mail::send_message(*send.clone(), send.json_file).await;
            match result {
                Err(e) => {
                    println!("{:?}", e)
                }
                Ok(_res) => {}
            };
        }
        Commands::Labels => {
            let labels_btreemap = mail::list_labels(&hub).await;
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
            let file_lock = Arc::new(Mutex::new(0));
            let mut dequerer_threads: Vec<tokio::task::JoinHandle<usize>> =
                Vec::with_capacity((filter.threads).try_into().unwrap());
            let mut enquerer_threads: Vec<tokio::task::JoinHandle<usize>> =
                Vec::with_capacity((filter.threads).try_into().unwrap());

            for _ in 0..filter.threads {
                let hub_clone = hub.clone();
                let msg_id_bts_clone = msg_id_bts.clone();
                let output_file = filter.output.clone();
                let file_lock_clone = file_lock.clone();
                let dequeue_thread = tokio::spawn(async move {
                    mail::print_msgs(&hub_clone, &MSG_ID_RB, output_file, file_lock_clone).await
                });
                let enqueue_thread =
                    tokio::spawn(async move { mail::add_msgs(msg_id_bts_clone, &MSG_ID_RB).await });
                dequerer_threads.push(dequeue_thread);
                enquerer_threads.push(enqueue_thread);
            }

            get_msg_ids_from_messages(&hub, None, Some(filter.filter), msg_id_bts.clone()).await;

            for _ in 0..filter.threads {
                let mut msg_id_bts_lock = msg_id_bts.lock().await;
                msg_id_bts_lock.insert(None);
            }

            MSG_ID_RB.poison().await;

            let mut messages_found: usize = 0;
            let mut messages_printed: usize = 0;
            while let Some(curr_thread) = dequerer_threads.pop() {
                messages_printed += curr_thread.await.unwrap();
            }

            while let Some(curr_thread) = enquerer_threads.pop() {
                messages_found += curr_thread.await.unwrap();
            }

            assert_eq!(messages_found, messages_printed);
            println!("Found {} messages!", messages_found);
        }
    }

    return;
}
