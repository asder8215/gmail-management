# Gmail-Management

This is a program that allows you to interact with your gmail and perform services on it like trashing an email or sending an email either through gmail or a third party mail service that uses SMTP.

# Inspiration

My friend [Megadash452](https://github.com/Megadash452) has always been telling me about his projects in Rust and what makes it a good language. I've never coded in Rust (I've coded in C/C++, Python, JS, Kotlin, Java) before, so I had a harder time understanding my friend's projects. Ownership policies are new to me as well, so I wanted to learn all about that and apply it in Rust. And through learning Rust, I could work alongside my friend with a project that we come up with. 

So learning Rust was on my mind firstly. Initially, I was going through the exercises in `rustlings` (I'll likely come back to it), but an idea came up in my head when I looked at how cluttered my email. I wanted to trash all those emails without having to manually do it. However, Gmail filters, as far as I'm aware, only have automated immediate deletes instead of by time intervals. There are certain emails in my inbox where I'd preferred them to be deleted the day after I see them instead of immediately (e.g. notifications from channels). With that thought, I decided that I would make a project working with the Gmail API in Rust. With being able to trash emails through the program, I could set up a cron job or some sort of task scheduler to run this program and trash emails from specific labels either nightly or on shutdown. I ended up expanding the program seeing labels in my email, sending emails, and possibly more features to add.

## Features

Note: anything with [] brackets are optionals, <> brackets are required, | symbols means or one of these (in `send` command, you need to at least specify a to, cc, or bcc address, but it's not necessary to use all three), {} are subcommands to the commands

- `trash [NUM_THREADS] {by-labels|by-msg-ids|by-filter}`: allows user to trash all emails in specific gmail label(s), a series of message IDs, or with a query filter
    - This command is multithreaded allowing users to specify between 1-10 threads respectively for enqueuing and dequeuing messages to trash emails from their inbox. As a result, the concurrency of fetching the message ids of the email and trashing the email through Gmail API allows you to clean your inbox efficiently.
- `labels`: allows user to see all labels within their gmail
- `send <<FROM> <TO|CC|BCC> <SUBJECT> [DESCRIPTION] [USER] [PASS] [ATTACHMENT] | <JSON FILE>> <RELAY>`: allows user to send an email with attachments through a mail service that uses SMTP
    - In order to use gmail as your relay, you must make sure your SMTP relay service settings are configured properly. This requires you to sign in with an Google admin account. If everything is set up appropriately, your relay would be `smtp.gmail.com:587` and you would put your gmail user in the username flag & gmail password in the password flag. See [Google's SMTP routing](https://support.google.com/a/answer/2956491?hl=en) for more info.
    - Alternatively, you can use other third party mail services that send emails via SMTP using TLS (e.g. Mailtrap).
    - With attachment option, you need to specify the file you want to attach to the email using the file path relative to where you run this program. 
    - For convenience, a `credentials.json` is stored locally on your PC when you login to the relay host for the first time. `credentials.json` stores the last username and password you logged in with that specific relay so that the next time you try to use the `send` command with the same relay, it's not necessary for you to put values in the --username (-u) & --password (-p) options
    - Emails details can be sent through a json file formatted with required info similarly to manually sending with the options. 
- `filter [NUM_THREADS] [query filter]`: allows user to query a search on their gmail inbox and receive an email blurbs related to the query within desired txt file
    - See `help filter` for all query filters possible. Also see [Google's Refined Searches](https://support.google.com/mail/answer/7190?hl=en) for more detail on gmail search queries.
    - This command is multithreaded as well allowing between 1-10 threads for enqueuing and dequeuing messages to ensure fast printing of messages into a given output file.
- `help [COMMAND]`: list all the commands provided by the program with a small blurb of what they do.
    - Specifying a command (e.g. `help send`) allows users to see more information about what the command takes and what each of the options in the command are meant for.

## Future Additions

- Allow `filter` command to output messages into a json file format
- Possibly have AI filtering of messages that sort the message information by relevancy or some other standards. Might be easier to have information to be parsed by a json file format than txt though.

## Download

You can download this program through the [Releases](https://github.com/asder8215/gmail-management/releases) page.

## Building & Running
Because an OAuth consent screen is necessary on first use of this program to access your Gmail, you need to generate a Client ID through Google Cloud console. Follow these [Get your Google API client ID](https://developers.google.com/identity/gsi/web/guides/get-google-api-clientid) steps that Google provides to generate this Client ID. Make sure to enable the Gmail API in the Google API project that you create. Once you have generated and downloaded the json file containing the Client ID, rename the file to `client_secret.json` and you're good to go! 

You may want to also put the Google Cloud Platform project from testing into production so that the refresh token provided by authentication doesn't expire every 7 days. You can see how to publish your Google Cloud Platform project from [Prepare and submit for verification](https://support.google.com/cloud/answer/13461325?hl=en#:~:text=For%20TV%20&%20Devices%20apps,then%20click%20%E2%80%9CSave%20and%20Continue%E2%80%9D) steps.

Run `cargo build` to build the project and `cargo run -- [command]` to run the project.

If this error, `“failed to run custom build command for aws-lc-sys”` occurs, it's possible that you may need to install `nasm` & `cmake`. If so, you can follow this [Medium article](https://medium.com/@rrnazario/rust-how-to-fix-failed-to-run-custom-build-command-for-aws-lc-sys-on-windows-c3bd2405ac6f) in order to fix the issue.

## Contribution

Credits to [Megadash452](https://github.com/Megadash452) for helping me out with understanding Rust code semantics & especially with understanding how to do multithreading in this program (my prior knowledge of doing multithreading is through C using pthreads). He helped me out tremendously in creating a generic multithreaded ring buffer (though it wasn't necessary, it helped me out in learning how to code in Rust) as we idealized about it to be used for this program and any other multithreading purposes that uses a ring buffer. [Generic Ring Buffers](https://github.com/asder8215/Generic-Ring-Buffer-Data-Structures) is a repository that will contain more polished versions of the multithreaded ring buffer data structure used in this project (alongside other variants of a ring buffer, const generic, regular generic multithreaded ring buffer, lock free const/regular generic ring buffer, const/regular generic single threaded ring buffer). 

## License

Gmail-management itself is licensed under the [MIT license](LICENSE) and includes this as the default project license.