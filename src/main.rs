extern crate google_gmail1 as gmail1;
pub mod mail_service;
pub mod ringbuffer;

use clap::Parser;
use mail_service as mail;
use ringbuffer::MultiThreadedRingBuffer;
use std::fmt::Debug;

/// Email management program that provides options in interacting with your gmail and send emails through a mail sending service
#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    cmds: Commands,
}

#[derive(Parser, Debug, Clone, PartialEq)]
enum Commands {
    /// Trashes email within specified label(s) or specified message(s) in authenticated email
    Trash {
        /// Trash all emails within specified label(s)
        #[arg(
            short,
            long,
            value_name = "LABEL_NAMES",
            required_unless_present("msgs")
        )]
        labels: Vec<String>,
        /// Trash all messages within specified message id(s)
        #[arg(
            short,
            long,
            value_name = "MESSAGE_ID",
            required_unless_present("labels")
        )]
        msgs: Vec<String>,
        /// The number of threads desired by the user to trash emails. Limited between 1 to 10 threads inclusive.
        #[arg(
            short,
            long,
            value_name = "NUM",
            default_value_t = 1,
            value_parser(1..11),
        )]
        threads_num: i64,
    },
    /// Sends an email to specified email address(es)
    Send {
        /// The email address you're sending from
        #[arg(short, long, value_name = "EMAIL_ADDR")]
        from: String,

        /// The email addresses you want to directly send an email to
        #[arg(short, long, value_name="EMAIL_ADDRS", required_unless_present_any(["cc", "bcc"]), num_args=1..100)]
        to: Vec<String>,

        /// The email addresses placed in the cc field you want to send an email to
        #[arg(short, long, value_name="EMAIL_ADDRS", required_unless_present_any(["to", "bcc"]), num_args=1..100)]
        cc: Vec<String>,

        /// The email addresses placed in the bcc field you want to send an email to
        #[arg(short, long, value_name="EMAIL_ADDRS", required_unless_present_any(["to", "cc"]), num_args=1..100)]
        bcc: Vec<String>,

        /// The subject of the email
        #[arg(short, long, value_name = "SUBJECT")]
        subject: Option<String>,

        /// The body description of the email
        #[arg(short, long, alias = "desc", value_name = "DESCRIPTION")]
        description: Option<String>,

        /// The attachment(s) of the email relative to the path of the attachment(s) file
        /// Keep in mind of the size of the attachments together when sending an email to individuals
        #[arg(short, long, alias = "path", value_name = "ATTACHMENTS")]
        attachment: Option<Vec<String>>,

        /// The username to the host website you're using to send an email from. Credentials for user is stored in credentials.json
        #[arg(short, long, value_name = "USERNAME", requires("password"))]
        username: Option<String>,

        /// The password to the host website you're using to send an email from. Credentials for pass is stored in credentials.json
        #[arg(short, long, value_name = "PASSWORD", requires("username"))]
        password: Option<String>,

        /// The host site that will provide you a method to send an email
        #[arg(short, long, value_name = "HOST SITE")]
        relay: String,
    },
    /// List all labels within authenticated email
    Labels,
}

#[tokio::main]
async fn main() {
    static MSG_ID_RB: MultiThreadedRingBuffer<String, 1024> = MultiThreadedRingBuffer::new();
    let hub = mail::create_client().await.unwrap();
    let args = Args::parse();

    // println!("Args: {args:?}");

    match args.cmds {
        Commands::Trash {
            labels,
            msgs,
            threads_num,
        } => {
            // Thread reference: https://doc.rust-lang.org/std/thread/
            let mut threads: Vec<tokio::task::JoinHandle<usize>> =
                Vec::with_capacity(threads_num.try_into().unwrap());

            for _ in 0..threads_num {
                let hub_clone = hub.clone();
                let handler =
                    tokio::spawn(async move { mail::trash_msgs(&hub_clone, &MSG_ID_RB).await });
                threads.push(handler);
            }

            // println!("Attempting to trash from inbox");
            if !labels.is_empty() {
                mail::trash_messages_from_labels(&hub, labels, &MSG_ID_RB).await;
            }
            if !msgs.is_empty() {
                mail::trash_messages_from_id(&hub, msgs, &MSG_ID_RB).await;
            }

            MSG_ID_RB.poison().await;

            let mut messages_taken: usize = 0;
            while let Some(curr_thread) = threads.pop() {
                messages_taken += curr_thread.await.unwrap();
            }

            println!("Taken {} messages!", messages_taken);
        }
        Commands::Send {
            username,
            password,
            relay,
            from,
            to,
            cc,
            bcc,
            subject,
            description,
            attachment,
        } => {
            let result = mail::send_message(
                username,
                password,
                relay,
                from,
                to,
                cc,
                bcc,
                subject,
                description,
                attachment,
            )
            .await;
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
    }

    return;
}
