extern crate google_gmail1 as gmail1;
extern crate getopts;

use getopts::Options;
use gmail1::hyper::client::HttpConnector;
use gmail1::hyper_rustls::HttpsConnector;
use gmail1::Error;
use gmail1::{Gmail, oauth2, hyper, hyper_rustls};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::process;
use clap::Parser;


/// Attempts to authenticate and connect to user's email; returns the connected client on success
/// Need to create a service account on Google Cloud Platform Console and put the client id in a client_secret.json
/// You can follow this for more info: https://support.google.com/cloud/answer/6158849?hl=en#:~:text=Go%20to%20the%20Google%20Cloud%20Platform%20Console%20Credentials%20page.,to%20add%20a%20new%20secret.
/// Much of this code inspired from: https://docs.rs/google-gmail1/latest/google_gmail1/index.html
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

/// Return a HashSet of all email message id
async fn list_messages() -> HashSet<String> {
    let hub = create_client().await;

    let (result, messages) = hub
        .users()
        .messages_list("me")
        .add_scope("https://mail.google.com/")
        .doit()
        .await
        .unwrap();

    // TODO: Find a way to error check on result 
    // assert_eq!(result.status(), StatusCode::);

    let messages = messages.messages.unwrap();
    let mut message_set = HashSet::<String>::default();

    for msg in messages {
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
async fn list_messages_by_label(label_id: &str) -> HashSet<String> {
    let hub = create_client().await;

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

        let page_token = messages.next_page_token.clone();
        // println!("{:?}", &page_token.unwrap());

        let msgs = messages.messages.clone().unwrap();

        for msg in msgs {
            message_set.insert(msg.id.clone().unwrap());
        }
        
        if page_token == None {
            fetch_emails = false;
        }
        else{
            (result, messages) = hub
                    .users()
                    .messages_list("me")
                    .page_token(&page_token.unwrap())
                    .add_label_ids(label_id)
                    .add_scope("https://mail.google.com/")
                    .doit()
                    .await
                    .unwrap();
        }
    }

    message_set
}

/// Return a HashMap of label names and ids within user's email
async fn list_labels() -> HashMap<String, String> {
    let hub = create_client().await;

    let (result, labels_list) = hub
            .users()
            .labels_list("me")
            .doit()
            .await
            .unwrap();
    let labels = labels_list.labels.unwrap();
    // println!("{:?}", &labels);
    let mut label_map = HashMap::<String, String>::default();
    for label in labels {
        label_map.insert(label.name.clone().unwrap(), label.id.clone().unwrap());
        // println!("{:?}", &label.name.unwrap());
        // println!("{:?}", &label.id.unwrap());
    }

    label_map
}

async fn get_label_id(label_name: String) -> Option<String>{
    // println!("{:?}", &label_name);
    let label_id = list_labels().await.get(&label_name).cloned();
    
    label_id
}

async fn trash_messages_from_label(label_name: String){
    let hub = create_client().await;

    let label_id = get_label_id(label_name).await.clone();

    if label_id == None {
        println!("Nonexistent label name");
        process::exit(1);
    }

    let list_of_msgs = list_messages_by_label(&label_id.unwrap()).await;

    for msg_id in list_of_msgs {
            let (result, trash_msg) = hub
                    .users()
                    .messages_trash("me", &msg_id)
                    .doit()
                    .await
                    .unwrap();
    }

}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} FILE [options]", program);
    print!("{}", opts.usage(&brief));
}

/// Gmail management program that provides options in interacting with your gmail
#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    #[command(subcommand)]
    delete: TrashCommand,

    /// List all labels in authenticated gmail
    #[arg(short, long)]
    label_list: bool,
}


#[derive(Parser, Debug)]
enum TrashCommand {
    /// Trashes email within specified labels or specified messages in authenticated email
    Trash{
        /// Trash all emails within specified labels
        #[arg(long, value_name="LABEL_NAMES")]
        labels: Option<Vec<String>>,
        /// Trash all messages within specified message ids
        #[arg(long, value_name="MESSAGE_ID")]
        messages: Option<Vec<String>>
    }
}

impl TrashCommand {
    fn retrieve_labels(&self) -> &Option<Vec<String>> {
        match self {
            Self::Trash { labels, ..} => labels,
        }
    }
    fn retrieve_messages(&self) -> &Option<Vec<String>> {
        match self {
            Self::Trash { messages, ..} => messages,
        }
    }
}


#[tokio::main]
async fn main() {
    
    // println!("{:?}", list_labels().await);

    // let args: Vec<String> = env::args().map(|x| x.to_string()).collect();
    // let ref program = args[0];

    // let mut opts = Options::new();
    // opts.optflag("h", "help", "Display help message.");
    // opts.optopt("d", "delete", "Delete emails given an id", "[-l] <Label Name | Message ID>");
    // opts.optflag("l", "labels", "List all labels within the authenticated email");

    // let matches = match opts.parse(&args[1..]){
    //     Ok(m) => { m }
    //     Err(f) => {
    //         println!("{}", f);
    //         process::exit(1);
    //     }
    // };

    // if matches.opt_present("h"){
    //     print_usage(&program, opts);
    //     return;
    // }
    // if matches.opt_present("d"){
    //     // let flag_or_id = matches.opt_str("d");
    //     // println!("{:?}", flag_or_id.clone().unwrap());
    //     // println!("{:?}", matches.opt_str("d").clone().unwrap());
    //     println!("{:?}", args);
    //     // println!("Argument {:?}, Position {:?}", matches.opt_strs("d"), matches.free);
    //     // match flag_or_id.clone().unwrap().as_str(){
    //     //     "l" => {trash_messages_from_label(matches.free.join(" ")).await},
    //     //     _ => println!("It's something else!")
    //     // };
    // }
    // if matches.opt_present("l"){

    // }

    let args = Args::parse();

    println!("Args: {:?},  Trash Labels: {:?}, Trash Messages: {:?}, Label-Flag: {:?}", args, args.delete.retrieve_labels(), args.delete.retrieve_messages(), args.label_list);

    return;
}
