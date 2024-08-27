use clap::{Parser, Subcommand};
use serde::{self, Deserialize, Serialize};
use std::fmt::Debug;

/// Email management program that provides options in interacting with your gmail and send emails through a mail sending service
#[derive(Parser, Debug)]
pub struct Args {
    #[command(subcommand)]
    pub cmds: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Trashes email within specified label(s) or specified message(s) in authenticated email
    Trash(Trash),
    /// Sends an email to specified email address(es)
    Send(Box<Send>),
    /// List all labels within authenticated email
    Labels,
    /// Filters messages in authenticated email and outputs them in a txt file.
    /// See Google's "Refine searches in Gmail" for more info on email search query
    Filter(Box<FilterWithOutput>),
}

#[derive(Parser, Debug)]
pub struct Trash {
    #[command(subcommand)]
    pub trash_opt: TrashOptions,

    /// The number of threads desired by the user to trash emails. Limited between 1 to 10 threads inclusive.
    #[arg(
        short,
        long,
        value_name = "NUM",
        default_value_t = 1,
        value_parser(1..11),
    )]
    pub threads_num: i64,
}

#[derive(Subcommand, Debug)]
pub enum TrashOptions {
    /// Trash all messages by message ids
    ByMsgIds(MsgIds),
    /// Trash all messages by label names
    ByLabels(Labels),
    /// Trash all messages by filter query.
    /// See Google's "Refine searches in Gmail" for more info on email search query
    ByFilter(Box<Filter>),
}

#[derive(Parser, Debug, Serialize, Deserialize)]
pub struct MsgIds {
    /// Message IDs of emails
    #[arg(short, long, value_name = "MESSAGE_ID")]
    pub msg_ids: Vec<String>,
}

#[derive(Parser, Debug, Serialize, Deserialize)]
pub struct Labels {
    /// Label names within user's email
    #[arg(short, long, value_name = "LABEL_NAMES")]
    pub labels: Vec<String>,
}

#[derive(Parser, Debug, Serialize, Deserialize, Clone)]
pub struct Send {
    /// The email address you're sending from
    #[arg(
        short,
        long,
        value_name = "EMAIL_ADDR",
        requires("recipient"),
        requires("subject"),
        group = "send_details"
    )]
    #[serde(default)]
    pub from: Option<String>,

    /// The email addresses you want to directly send an email to
    #[arg(short, long, value_name="EMAIL_ADDRS", requires("from"), num_args=1..100, group="recipient")]
    #[serde(default)]
    pub to: Option<Vec<String>>,

    /// The email addresses placed in the cc field you want to send an email to
    #[arg(short, long, value_name="EMAIL_ADDRS", requires("from"), num_args=1..100, group="recipient")]
    #[serde(default)]
    pub cc: Option<Vec<String>>,

    /// The email addresses placed in the bcc field you want to send an email to
    #[arg(short, long, value_name="EMAIL_ADDRS", requires("from"), num_args=1..100, group="recipient")]
    #[serde(default)]
    pub bcc: Option<Vec<String>>,

    /// The subject of the email
    #[arg(short, long, value_name = "SUBJECT", requires("from"))]
    #[serde(default)]
    pub subject: Option<String>,

    /// The body description of the email
    #[arg(
        short,
        long,
        alias = "desc",
        value_name = "DESCRIPTION",
        requires("from")
    )]
    #[serde(default)]
    pub description: Option<String>,

    /// The attachment(s) of the email relative to the path of the attachment(s) file
    /// Keep in mind of the size of the attachments together when sending an email to individuals
    #[arg(
        short,
        long,
        alias = "path",
        value_name = "ATTACHMENTS",
        requires("from")
    )]
    #[serde(default)]
    pub attachment: Option<Vec<String>>,

    /// The username to the host website you're using to send an email from. Credentials for user is stored in credentials.json
    #[arg(short, long, value_name = "USERNAME", requires("password"))]
    #[serde(default)]
    pub username: Option<String>,

    /// The password to the host website you're using to send an email from. Credentials for pass is stored in credentials.json
    #[arg(short, long, value_name = "PASSWORD", requires("username"))]
    #[serde(default)]
    pub password: Option<String>,

    /// The host site that will provide you a method to send an email
    #[arg(short, long, value_name = "HOST SITE", requires("send_details"))]
    #[serde(default)]
    pub relay: String,

    /// Input json file containing message of the email. If provided, this takes precedence.
    #[arg(short, long, value_name = "JSON FILE", group = "send_details")]
    #[serde(default)]
    pub json_file: Option<String>,
}

#[derive(Parser, Debug, Serialize, Deserialize, Clone)]
pub struct SendInfo {
    /// The email address you're sending from
    #[arg(
        short,
        long,
        value_name = "EMAIL_ADDR",
        requires("recipient"),
        group = "manual_send"
    )]
    #[serde(default)]
    pub from: Option<String>,

    /// The email addresses you want to directly send an email to
    #[arg(short, long, value_name="EMAIL_ADDRS", requires("from"), num_args=1..100, group="recipient")]
    #[serde(default)]
    pub to: Option<Vec<String>>,

    /// The email addresses placed in the cc field you want to send an email to
    #[arg(short, long, value_name="EMAIL_ADDRS", requires("from"), num_args=1..100, group="recipient")]
    #[serde(default)]
    pub cc: Option<Vec<String>>,

    /// The email addresses placed in the bcc field you want to send an email to
    #[arg(short, long, value_name="EMAIL_ADDRS", requires("from"), num_args=1..100, group="recipient")]
    #[serde(default)]
    pub bcc: Option<Vec<String>>,

    /// The subject of the email
    #[arg(short, long, value_name = "SUBJECT", requires("from"))]
    #[serde(default)]
    pub subject: Option<String>,

    /// The body description of the email
    #[arg(
        short,
        long,
        alias = "desc",
        value_name = "DESCRIPTION",
        requires("from")
    )]
    #[serde(default)]
    pub description: Option<String>,

    /// The attachment(s) of the email relative to the path of the attachment(s) file
    /// Keep in mind of the size of the attachments together when sending an email to individuals
    #[arg(
        short,
        long,
        alias = "path",
        value_name = "ATTACHMENTS",
        requires("from")
    )]
    #[serde(default)]
    pub attachment: Option<Vec<String>>,
}

#[derive(Subcommand, Debug)]
pub enum FilterCmd {
    Filter(Filter),
}

/// Filters messages in authenticated email for trashing purposes
/// See Google's "Refine searches in Gmail" for more info on email search query
#[derive(Parser, Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Filter {
    /// Messages that contain these specific word(s) or phrase(s)
    #[arg(short, long, value_name = "WORDS")]
    #[serde(default)]
    pub words: Option<Vec<String>>,

    /// Messages with these specific from address(es)
    #[arg(long, value_name = "FROM")]
    #[serde(default)]
    pub from: Option<Vec<String>>,

    /// Messages with these specific to address(es)
    #[arg(long, value_name = "TO")]
    #[serde(default)]
    pub to: Option<Vec<String>>,

    /// Messages with these specific cc address(es)
    #[arg(long, value_name = "CC")]
    #[serde(default)]
    pub cc: Option<Vec<String>>,

    /// Messages with these specific bcc address(es)
    #[arg(long, value_name = "BCC")]
    #[serde(default)]
    pub bcc: Option<Vec<String>>,

    /// Messages with that contain these subject(s)
    #[arg(long, value_name = "SUBJECT")]
    #[serde(default)]
    pub subject: Option<Vec<String>>,

    /// Messages that do not contain these word(s)
    #[arg(short, long, value_name = "REMOVE-WORD")]
    #[serde(default)]
    pub remove_words: Option<Vec<String>>,

    /// Messages with specific label(s)
    #[arg(long, value_name = "LABEL_NAME")]
    #[serde(default)]
    pub labels: Option<Vec<String>>,

    /// Messages with certain icon(s)
    #[arg(long, value_name = "HAS")]
    #[serde(default)]
    pub has: Option<Vec<String>>,

    /// Messages with certain mailing list(s) an email is associated with
    #[arg(long, value_name = "LIST")]
    #[serde(default)]
    pub list: Option<Vec<String>>,

    /// Messages that contain attachment(s with a certain name(s) or filetype(s)
    #[arg(long, value_name = "FILENAME/TYPE")]
    #[serde(default)]
    pub filename: Option<Vec<String>>,

    /// Messages in certain folder(s)
    #[arg(long, value_name = "IN")]
    #[serde(default)]
    pub r#in: Option<Vec<String>>,

    /// Messages that may be starred, snoozed, unread, read, or muted
    #[arg(long, value_name = "IS")]
    #[serde(default)]
    pub is: Option<Vec<String>>,

    /// Messages found after a certain date
    #[arg(long, value_name = "AFTER-DATE")]
    #[serde(default)]
    pub after: Option<String>,

    /// Messages found before a certain date
    #[arg(long, value_name = "BEFORE-DATE")]
    #[serde(default)]
    pub before: Option<String>,

    /// Messages older than a certain amount of time
    #[arg(long, value_name = "OLDER-THAN")]
    #[serde(default)]
    pub older_than: Option<String>,

    /// Messages newer than a certain amount of time
    #[arg(long, value_name = "NEWER-THAN")]
    #[serde(default)]
    pub newer_than: Option<String>,

    /// Messages deliver to certain address(es)
    #[arg(long, value_name = "DELIVERED TO")]
    #[serde(default)]
    pub deliveredto: Option<Vec<String>>,

    /// Messages given specific categorie(s)
    #[arg(long, value_name = "CATEGORY")]
    #[serde(default)]
    pub category: Option<Vec<String>>,

    /// Messages with a specific size or larger in bytes
    #[arg(long, value_name = "SIZE")]
    #[serde(default)]
    pub size: Option<usize>,

    /// Messages larger than a specific size in bytes
    #[arg(long, value_name = "LARGER-THAN")]
    #[serde(default)]
    pub larger: Option<usize>,

    /// Messages smaller than a specific size in bytes
    #[arg(long, value_name = "SMALLER-THAN")]
    #[serde(default)]
    pub smaller: Option<usize>,

    /// Messages with a certain message-id header
    #[arg(long, value_name = "RFC822MSGID")]
    #[serde(default)]
    pub rfc822msgid: Option<Vec<String>>,

    /// Input text file to search query on authenticated email
    #[arg(short, long, value_name = "TEXT FILE", exclusive = true)]
    #[serde(default)]
    pub text_file: Option<String>,

    /// Input json file to search query on authenticated email
    #[arg(short, long, value_name = "JSON FILE", exclusive = true)]
    #[serde(default)]
    pub json_file: Option<String>,
}

/// Filters messages in authenticated email and outputs them in a txt file.
/// See Google's "Refine searches in Gmail" for more info on email search query
#[derive(Parser, Debug, Clone, Serialize, Deserialize)]
pub struct FilterWithOutput {
    #[clap(flatten)]
    pub filter: Filter,

    /// Output txt file name that contains all filtered messages
    #[arg(short, long, value_name = "OUTPUT FILE")]
    pub output: String,

    /// The number of threads desired by the user to trash emails. Limited between 1 to 10 threads inclusive.
    #[arg(
        long,
        value_name = "NUM",
        default_value_t = 1,
        value_parser(1..11),
    )]
    pub threads: i64,
}
