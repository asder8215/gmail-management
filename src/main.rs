extern crate google_gmail1 as gmail1;
use gmail1::{api::ListMessagesResponse, hyper::client::HttpConnector};
use gmail1::hyper_rustls::HttpsConnector;
use gmail1::Error;
use std::collections::{HashMap, HashSet};
use std::fs;
use gmail1::{Gmail, oauth2, hyper, hyper_rustls};



/// Attempts to authenticate and connect to user's email; returns the connected client on success
/// Need to create a service account on Google Cloud Platform Console and put the client id in a client_secret.json
/// You can follow this for more info: https://support.google.com/cloud/answer/6158849?hl=en#:~:text=Go%20to%20the%20Google%20Cloud%20Platform%20Console%20Credentials%20page.,to%20add%20a%20new%20secret.
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

    hub
}

/// Return a HashSet of all email message id
async fn list_messages() -> HashSet<String> {
    let hub = create_client().await;

    let (result, messages) = hub
        .users()
        .messages_list("me")
        // .add_label_ids("Label_1761780972973684878")
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


/// Return a HashMap of label names and ids within user's email
async fn list_labels() -> HashMap<String, String> {
    let hub = create_client().await;

    let (result, labels_list) = hub.users().labels_list("me").doit().await.unwrap();
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


#[tokio::main]
async fn main() {
    
    // let hub = create_client().await;
    
    // println!("{:?}", &get_msg);
    // let labels = hub.users().labels_list("me").doit().await.unwrap().1;
    // let labels = labels.labels.unwrap();
    // println!("{:?}", &labels);
    // let mut label_map = HashMap::<String, String>::default();
    // for label in labels {
    //     label_map.insert(label.name.clone().unwrap(), label.id.clone().unwrap());
    //     println!("{:?}", &label.name.unwrap());
    //     println!("{:?}", &label.id.unwrap());
    // }

    return;

}
