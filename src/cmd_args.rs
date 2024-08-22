use clap::{Parser, Subcommand};
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
    Send(Send),
    /// List all labels within authenticated email
    Labels,
    /// Filters messages in authenticated email and outputs them in a json file.
    /// See Google's "Refine searches in Gmail" for more info on email search query
    Filter(FilterWithOutput),
    /// Trash messages in authenticated email based on filters provided by user.
    /// See Google's "Refine searches in Gmail" for more info on email search query
    FilterTrash(Filter),
}

#[derive(Parser, Debug)]
pub struct Trash {
    /// Trash all emails within specified label(s)
    #[arg(
        short,
        long,
        value_name = "LABEL_NAMES",
        required_unless_present("msgs")
    )]
    pub labels: Vec<String>,

    /// Trash all messages within specified message id(s)
    #[arg(
        short,
        long,
        value_name = "MESSAGE_ID",
        required_unless_present("labels")
    )]
    pub msgs: Vec<String>,

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

#[derive(Parser, Debug)]
pub struct Send {
    /// The email address you're sending from
    #[arg(short, long, value_name = "EMAIL_ADDR")]
    pub from: String,

    /// The email addresses you want to directly send an email to
    #[arg(short, long, value_name="EMAIL_ADDRS", required_unless_present_any(["cc", "bcc"]), num_args=1..100)]
    pub to: Vec<String>,

    /// The email addresses placed in the cc field you want to send an email to
    #[arg(short, long, value_name="EMAIL_ADDRS", required_unless_present_any(["to", "bcc"]), num_args=1..100)]
    pub cc: Vec<String>,

    /// The email addresses placed in the bcc field you want to send an email to
    #[arg(short, long, value_name="EMAIL_ADDRS", required_unless_present_any(["to", "cc"]), num_args=1..100)]
    pub bcc: Vec<String>,

    /// The subject of the email
    #[arg(short, long, value_name = "SUBJECT")]
    pub subject: Option<String>,

    /// The body description of the email
    #[arg(short, long, alias = "desc", value_name = "DESCRIPTION")]
    pub description: Option<String>,

    /// The attachment(s) of the email relative to the path of the attachment(s) file
    /// Keep in mind of the size of the attachments together when sending an email to individuals
    #[arg(short, long, alias = "path", value_name = "ATTACHMENTS")]
    pub attachment: Option<Vec<String>>,

    /// The username to the host website you're using to send an email from. Credentials for user is stored in credentials.json
    #[arg(short, long, value_name = "USERNAME", requires("password"))]
    pub username: Option<String>,

    /// The password to the host website you're using to send an email from. Credentials for pass is stored in credentials.json
    #[arg(short, long, value_name = "PASSWORD", requires("username"))]
    pub password: Option<String>,

    /// The host site that will provide you a method to send an email
    #[arg(short, long, value_name = "HOST SITE")]
    pub relay: String,
}

#[derive(Subcommand, Debug)]
pub enum FilterCmd {
    Filter(Filter),
}

/// Filters messages in authenticated email and outputs them in a json file.
/// See Google's "Refine searches in Gmail" for more info on email search query
#[derive(Parser, Debug)]
pub struct FilterWithOutput {
    /// Messages that contain these specific word(s) or phrase(s)
    #[arg(short, long, value_name = "WORDS")]
    pub words: Option<Vec<String>>,

    /// Messages with these specific from address(es)
    #[arg(long, value_name = "FROM")]
    pub from: Option<Vec<String>>,

    /// Messages with these specific to address(es)
    #[arg(long, value_name = "TO")]
    pub to: Option<Vec<String>>,

    /// Messages with these specific cc address(es)
    #[arg(long, value_name = "CC")]
    pub cc: Option<Vec<String>>,

    /// Messages with these specific bcc address(es)
    #[arg(long, value_name = "BCC")]
    pub bcc: Option<Vec<String>>,

    /// Messages with that contain these subject(s)
    #[arg(long, value_name = "SUBJECT")]
    pub subject: Option<Vec<String>>,

    /// Messages that do not contain these word(s)
    #[arg(short, long, value_name = "REMOVE-WORD")]
    pub remove_words: Option<Vec<String>>,

    /// Messages with specific label(s)
    #[arg(long, value_name = "LABEL_NAME")]
    pub labels: Option<Vec<String>>,

    /// Messages with certain icon(s)
    #[arg(long, value_name = "HAS")]
    pub has: Option<Vec<String>>,

    /// Messages with certain mailing list(s) an email is associated with
    #[arg(long, value_name = "LIST")]
    pub list: Option<Vec<String>>,

    /// Messages that contain attachment(s with a certain name(s) or filetype(s)
    #[arg(long, value_name = "FILENAME/TYPE")]
    pub filename: Option<Vec<String>>,

    /// Messages in certain folder(s)
    #[arg(long, value_name = "IN")]
    pub r#in: Option<Vec<String>>,

    /// Messages that may be starred, snoozed, unread, read, or muted
    #[arg(long, value_name = "IS")]
    pub is: Option<Vec<String>>,

    /// Messages found after a certain date
    #[arg(long, value_name = "AFTER-DATE")]
    pub after: Option<String>,

    /// Messages found before a certain date
    #[arg(long, value_name = "BEFORE-DATE")]
    pub before: Option<String>,

    /// Messages older than a certain amount of time
    #[arg(long, value_name = "OLDER-THAN")]
    pub older_than: Option<String>,

    /// Messages newer than a certain amount of time
    #[arg(long, value_name = "NEWER-THAN")]
    pub newer_than: Option<String>,

    /// Messages deliver to certain address(es)
    #[arg(long, value_name = "DELIVERED TO")]
    pub deliveredto: Option<Vec<String>>,

    /// Messages given specific categorie(s)
    #[arg(long, value_name = "CATEGORY")]
    pub category: Option<Vec<String>>,

    /// Messages with a specific size or larger in bytes
    #[arg(long, value_name = "SIZE")]
    pub size: Option<usize>,

    /// Messages larger than a specific size in bytes
    #[arg(long, value_name = "LARGER-THAN")]
    pub larger: Option<usize>,

    /// Messages smaller than a specific size in bytes
    #[arg(long, value_name = "SMALLER-THAN")]
    pub smaller: Option<usize>,

    /// Messages with a certain message-id header
    #[arg(long, value_name = "RFC822MSGID")]
    pub rfc822msgid: Option<Vec<String>>,

    /// Input text file to search query on authenticated email
    #[arg(short, long, value_name = "TEXT FILE", exclusive = true)]
    pub text_file: Option<String>,

    /// Input json file to search query on authenticated email
    #[arg(short, long, value_name = "JSON FILE", exclusive = true)]
    pub json_file: Option<String>,

    /// Output json file name that contains all filtered messages
    #[arg(short, long, value_name = "OUTPUT FILE")]
    pub output: String,
}

/// Filters messages in authenticated email for trashing purposes
/// See Google's "Refine searches in Gmail" for more info on email search query
#[derive(Parser, Debug)]
pub struct Filter {
    /// Messages that contain these specific word(s) or phrase(s)
    #[arg(short, long, value_name = "WORDS")]
    pub words: Option<Vec<String>>,

    /// Messages with these specific from address(es)
    #[arg(long, value_name = "FROM")]
    pub from: Option<Vec<String>>,

    /// Messages with these specific to address(es)
    #[arg(long, value_name = "TO")]
    pub to: Option<Vec<String>>,

    /// Messages with these specific cc address(es)
    #[arg(long, value_name = "CC")]
    pub cc: Option<Vec<String>>,

    /// Messages with these specific bcc address(es)
    #[arg(long, value_name = "BCC")]
    pub bcc: Option<Vec<String>>,

    /// Messages with that contain these subject(s)
    #[arg(long, value_name = "SUBJECT")]
    pub subject: Option<Vec<String>>,

    /// Messages that do not contain these word(s)
    #[arg(short, long, value_name = "REMOVE-WORD")]
    pub remove_words: Option<Vec<String>>,

    /// Messages with specific label(s)
    #[arg(long, value_name = "LABEL_NAME")]
    pub labels: Option<Vec<String>>,

    /// Messages with certain icon(s)
    #[arg(long, value_name = "HAS")]
    pub has: Option<Vec<String>>,

    /// Messages with certain mailing list(s) an email is associated with
    #[arg(long, value_name = "LIST")]
    pub list: Option<Vec<String>>,

    /// Messages that contain attachment(s with a certain name(s) or filetype(s)
    #[arg(long, value_name = "FILENAME/TYPE")]
    pub filename: Option<Vec<String>>,

    /// Messages in certain folder(s)
    #[arg(long, value_name = "IN")]
    pub r#in: Option<Vec<String>>,

    /// Messages that may be starred, snoozed, unread, read, or muted
    #[arg(long, value_name = "IS")]
    pub is: Option<Vec<String>>,

    /// Messages found after a certain date
    #[arg(long, value_name = "AFTER-DATE")]
    pub after: Option<String>,

    /// Messages found before a certain date
    #[arg(long, value_name = "BEFORE-DATE")]
    pub before: Option<String>,

    /// Messages older than a certain amount of time
    #[arg(long, value_name = "OLDER-THAN")]
    pub older_than: Option<String>,

    /// Messages newer than a certain amount of time
    #[arg(long, value_name = "NEWER-THAN")]
    pub newer_than: Option<String>,

    /// Messages deliver to certain address(es)
    #[arg(long, value_name = "DELIVERED TO")]
    pub deliveredto: Option<Vec<String>>,

    /// Messages given specific categorie(s)
    #[arg(long, value_name = "CATEGORY")]
    pub category: Option<Vec<String>>,

    /// Messages with a specific size or larger in bytes
    #[arg(long, value_name = "SIZE")]
    pub size: Option<usize>,

    /// Messages larger than a specific size in bytes
    #[arg(long, value_name = "LARGER-THAN")]
    pub larger: Option<usize>,

    /// Messages smaller than a specific size in bytes
    #[arg(long, value_name = "SMALLER-THAN")]
    pub smaller: Option<usize>,

    /// Messages with a certain message-id header
    #[arg(long, value_name = "RFC822MSGID")]
    pub rfc822msgid: Option<Vec<String>>,

    /// Input text file to search query on authenticated email
    #[arg(short, long, value_name = "TEXT FILE", exclusive = true)]
    pub text_file: Option<String>,

    /// Input json file to search query on authenticated email
    #[arg(short, long, value_name = "JSON FILE", exclusive = true)]
    pub json_file: Option<String>,
}
