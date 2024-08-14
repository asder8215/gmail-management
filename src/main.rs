extern crate google_gmail1 as gmail1;

use gmail1::hyper::client::HttpConnector;
use gmail1::hyper_rustls::HttpsConnector;
use gmail1::Error;
use gmail1::{Gmail, oauth2, hyper, hyper_rustls};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs; // for debugging purposes 
use clap::Parser;


/// Attempts to authenticate and connect to user's email; returns the connected client on success
/// Need to create a service account on Google Cloud Platform Console and put the client id in a client_secret.json
/// 
/// You can follow this for more info: [Google Cloud Help](https://support.google.com/cloud/answer/6158849?hl=en#:~:text=Go%20to%20the%20Google%20Cloud%20Platform%20Console%20Credentials%20page.,to%20add%20a%20new%20secret.)
/// 
/// Much of this code inspired from: [Google Gmail1 Doc](https://docs.rs/google-gmail1/latest/google_gmail1/index.html)
async fn create_client() -> Gmail<HttpsConnector<HttpConnector>> {
    // Get an ApplicationSecret instance by some means. It contains the `client_id` and
    // `client_secret`, among other things.

    let secret = oauth2::read_application_secret("./client_secret.json")
        .await
        .expect("client_secret.json");

    // Create an authenticator that uses an InstalledFlow to authenticate. The
    // authentication tokens are persisted to a file named tokencache.json. The
    // authenticator takes care of caching tokens to disk and refreshing tokens once
    // they've expired.
    let auth = oauth2::InstalledFlowAuthenticator::builder(
        secret,
        oauth2::InstalledFlowReturnMethod::HTTPRedirect,
    )
    .persist_tokens_to_disk("./tokens")
    .build()
    .await
    .unwrap();

    let https = hyper_rustls::HttpsConnectorBuilder::new().with_native_roots().unwrap().https_or_http().enable_http1().build();

    let hub = Gmail::new(
        hyper::Client::builder().build(https),
        auth,
    );

    // Test connection to see if user is authenticated and info can be retrieved
    let result = hub
        .users()
        .get_profile("me")
        .add_scope("https://mail.google.com/")
        .doit()
        .await;


    // Displays whether the result indicates a successful connection or a failed one
    match result {
        Err(e) => match e {
            // The Error enum provides details about what exactly happened.
            // You can also just use its `Debug`, `Display` or `Error` traits
             Error::HttpError(_)
            |Error::Io(_)
            |Error::MissingAPIKey
            |Error::MissingToken(_)
            |Error::Cancelled
            |Error::UploadSizeLimitExceeded(_, _)
            |Error::Failure(_)
            |Error::BadRequest(_)
            |Error::FieldClash(_)
            |Error::JsonDecodeError(_, _) => println!("{}", e),
        },
        Ok(res) => println!("Success: {:?}", res),
    };

    // print empty line for better readability of printed messages after fail/success
    println!("");

    hub
}

/// Fetches message from authenticated user's email given a message id
/// Returns None if the message is nonexistent
async fn get_messages(hub: &Gmail<HttpsConnector<HttpConnector>>, msg_ids: &String) -> Option<gmail1::api::Message>{
    let mut result = hub
        .users()
        .messages_get("me", msg_ids)
        .add_scope("https://mail.google.com/")
        .doit()
        .await;

    let mut message;
    match result {
        Err(e) => match e {
            // The Error enum provides details about what exactly happened.
            // You can also just use its `Debug`, `Display` or `Error` traits
                Error::HttpError(_)
            |Error::Io(_)
            |Error::MissingAPIKey
            |Error::MissingToken(_)
            |Error::Cancelled
            |Error::UploadSizeLimitExceeded(_, _)
            |Error::Failure(_)
            |Error::BadRequest(_)
            |Error::FieldClash(_)
            |Error::JsonDecodeError(_, _) => {
                println!("{}", e); 
                message = None;
            },
        },
        Ok(res) => {
            let (result, msg) = res;
            message = Some(msg);
        },
    };
    message
}

/// Return a HashSet of all email message ids
async fn list_messages(hub: &Gmail<HttpsConnector<HttpConnector>>) -> HashSet<String> {
    let (result, messages) = hub
        .users()
        .messages_list("me")
        .add_scope("https://mail.google.com/")
        .doit()
        .await
        .unwrap();

    // TODO: Find a way to error check on result 
    // assert_eq!(result.status(), StatusCode::);

    // let messages = messages.messages.unwrap();
    let mut message_set = HashSet::<String>::default();

    for msg in messages.messages.as_ref().unwrap() {
        message_set.insert(msg.id.clone().unwrap());
    }

    // Testing how many messages are retrieved and printing it out into the console
    // let message_count = messages.messages.clone().unwrap().iter().count();
    // let binding = messages.messages.clone().unwrap();
    // let message_iter = binding.iter().take(message_count).cloned();
    // for msg in message_iter {
    //     println!("{:?}", &msg);
    //     println!("{:?}", &message_count);
    //     return;
    // }


    // Further tests to see how the json files look like from the messages
    // fs::write(
    //     "test2.json",
    //     serde_json::to_string_pretty(&messages).unwrap(),
    // ).unwrap();

    // let (result, msg_retrieved) = hub.users().messages_get("me", "1912fbf6d84f437a").add_scope("https://mail.google.com/").doit().await.unwrap();

    // fs::write(
    //     "email_test.json",
    //     serde_json::to_string_pretty(&msg_retrieved).unwrap(),
    // ).unwrap();

    message_set
}

/// Return a HashSet of all email message id by label id
async fn list_messages_by_label(hub: &Gmail<HttpsConnector<HttpConnector>>, label_id: &str) -> HashSet<String> {
    let mut message_set = HashSet::<String>::default();
    let mut fetch_emails = true;

    let (mut result, mut messages) = hub
        .users()
        .messages_list("me")
        .add_label_ids(label_id)
        .add_scope("https://mail.google.com/")
        .doit()
        .await
        .unwrap();
    while fetch_emails == true {

        // TODO: Find a way to error check on result 
        // assert_eq!(result.status(), StatusCode::);

        // println!("{:?}", messages);

        // let page_token = messages.next_page_token.clone();
        // println!("{:?}", &page_token.unwrap());

        // let msgs = messages.messages;

        for msg in messages.messages.as_ref().unwrap() {
            message_set.insert(msg.id.clone().unwrap());
        }
        
        if let Some(page_token) = &messages.next_page_token {
            //
            (result, messages) = hub
                .users()
                .messages_list("me")
                .page_token(page_token)
                .add_label_ids(label_id)
                .add_scope("https://mail.google.com/")
                .doit()
                .await
                .unwrap();
        }
        else{
            fetch_emails = false;
        }
    }

    message_set
}

/// Return a BTreeMap of label names and ids within user's email
async fn list_labels(hub: &Gmail<HttpsConnector<HttpConnector>>) -> BTreeMap<String, String> {
    let (result, labels_list) = hub
        .users()
        .labels_list("me")
        .doit()
        .await
        .unwrap();
    // let labels = labels_list.labels.unwrap();
    // println!("{:?}", &labels);
    // let mut label_map = HashMap::<String, String>::default();
    let mut label_map = BTreeMap::<String, String>::default();
    for label in labels_list.labels.as_ref().unwrap() {
        label_map.insert(label.name.clone().unwrap(), label.id.clone().unwrap());
        // println!("{:?}", &label.name.unwrap());
        // println!("{:?}", &label.id.unwrap());
    }

    label_map
}

/// Retrieves label id given label name
async fn get_label_id(hub: &Gmail<HttpsConnector<HttpConnector>>, label_name: &String) -> Option<String>{
    // println!("{:?}", &label_name);
    let label_id = list_labels(hub).await.get(label_name).cloned();
    
    label_id
}

/// Trashes all emails from given labels
async fn trash_messages_from_labels(hub: &Gmail<HttpsConnector<HttpConnector>>, label_names: Vec<String>){
    for label in label_names {
        let label_id = get_label_id(hub, &label).await.clone();

        if label_id == None {
            println!("{} is a nonexistent label name", label);
            continue;
        }

        let list_of_msgs = list_messages_by_label(hub, &label_id.unwrap()).await;

        for msg_id in list_of_msgs {
                let (result, trash_msg) = hub
                        .users()
                        .messages_trash("me", &msg_id)
                        .doit()
                        .await
                        .unwrap();
        }
    }
}

/// Trash emails from provided message ids
async fn trash_messages_from_id(hub: &Gmail<HttpsConnector<HttpConnector>>, msg_ids: Vec<String>){
    for msg_id in msg_ids {

        if let Some(msg) = get_messages(hub, &msg_id).await {
            let (result, trash_msg) = hub
                .users()
                .messages_trash("me", &msg_id)
                .doit()
                .await
                .unwrap();
        }
        else{
            println!("{} is a nonexistent message id", msg_id);
            continue;
        }
    }
}

/// Gmail management program that provides options in interacting with your gmail
#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    cmds: Commands,
}

#[derive(Parser, Debug, Clone, PartialEq)]
enum Commands {
    /// Trashes email within specified label(s) or specified message(s) in authenticated email
    Trash{
        /// Trash all emails within specified label(s)
        #[arg(short, long, value_name="LABEL_NAMES")]
        labels: Option<Vec<String>>,
        /// Trash all messages within specified message id(s)
        #[arg(short, long, value_name="MESSAGE_ID")]
        msgs: Option<Vec<String>>
    },
    /// Sends an email to specified email address(es)
    Send {
        /// The email addresses you want to send an email to
        #[arg(short, long, value_name="EMAIL_ADDR")]
        to: Vec<String>,
    
        /// The subject of the email
        #[arg(short, long, value_name="SUBJECT")]
        subject: Option<String>,
    
        /// The description of the email
        #[arg(short, long, alias="desc", value_name="DESCRIPTION")]
        description: Option<String>
    },
    /// List all labels within authenticated email
    Labels,
}

#[tokio::main]
async fn main() {
    let hub = create_client().await;
    let args = Args::parse();
    // println!("Args: {args:?}");

    match args.cmds {
        Commands::Trash { 
            labels, msgs
        } => {
            if let Some(labels) = labels {
                trash_messages_from_labels(&hub, labels).await;
            }
            if let Some(msgs) = msgs {
                trash_messages_from_id(&hub, msgs).await;
            }
        },
        Commands::Send { to, subject, description } => todo!(),
        Commands::Labels => {
            let labels_hashmap = list_labels(&hub).await;
            let size = labels_hashmap.len();
            let mut count = 0;
            print!("All Labels in authenticated user's inbox: ");
            for label_id_pair in labels_hashmap{
                count += 1;
                if count != size {
                    print!("{}, ", label_id_pair.0);
                }
                else {
                    print!("{}", label_id_pair.0);
                }
            }
        },
    }
    
    // if trash != None{
    //     let trash = trash.unwrap();
    //     println!("Args: {:?},  Trash Labels: {:?}, Trash Messages: {:?}, Label-Flag: {:?}", args, trash.retrieve_labels(), trash.retrieve_messages(), args.label_list);
    // }
    // else {
    //     println!("Args: {:?}, Label-Flag: {:?}", args, args.label_list);
    // }

    return;
}
