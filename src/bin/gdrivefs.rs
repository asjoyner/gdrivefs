// A FUSE filesystem backed by google drive.
extern crate docopt;
extern crate env_logger;
extern crate fuse;
extern crate gdrivefs;
#[macro_use]
extern crate log;
extern crate rustc_serialize;
extern crate time;

use gdrivefs::oauth;

const USAGE: &'static str = "
gdrivefs: A fuse filesystem backed by Google Drive.

Usage:
  gdrivefs [options] <mountpoint>
  gdrivefs (-h | --help)

<mountpoint> must exist.

Several options can make a large performance difference, depending on the
workload and characteristics of the system. Setting 'read-block-multipler
to higher values will result in fewer HTTP requests, and less overhead per
byte, but can lead to increased latency for small random reads. Likewise,
enabling readahead can help on lower-memory systems where the OS chooses
not to do its own readahead on sequential reads, but can actually slow
down performance markedly on systems that perform their own readahead.

Options:
  -h --help                           Show this screen.
  --client-id-file=<id_file>          Path to a file containing the oauth2 client id. [default: /usr/local/etc/gdrive_id]
  --client-secret-file=<secret_file>  Path to a file containing the oauth2 client secret. [default: /usr/local/etc/gdrive_secret]
  --token-file=<token_file>           Path to a file containing a oauth token (generated by init_token). [default: /usr/local/etc/gdrive_token]
  --allow-other                       If true, allow non-root users to access the mounted filesystem.
  --dir-poll-secs=<poll-secs>         Seconds between directory refresh scans, or 0 to disable. [default: 900]
  --readahead-queue-size=<size>       Size of the readahead queue (per-file, in number of chunks), or 0 to disable. [default: 0]
  --file-read-cache-blocks=<size>     Capacity of the per-file chunk cache (in number of chunks). [default: 10]
  --read-block-multiplier=<mult>      Number of 4k blocks to read per HTTP request. [default: 2048]
";

#[derive(Debug, RustcDecodable)]
struct Args {
  flag_client_id_file: String,
  flag_client_secret_file: String,
  flag_token_file: String,
  flag_allow_other: bool,
  flag_dir_poll_secs: u32,
  flag_readahead_queue_size: usize,
  flag_file_read_cache_blocks: usize,
  flag_read_block_multiplier: u32,
  arg_mountpoint: String,
}

fn main() {
  env_logger::init().unwrap();

  let args: Args = docopt::Docopt::new(USAGE)
    .and_then(|d| d.decode())
    .unwrap_or_else(|e| e.exit());

  info!("Got args: {:?}", args);

  let client = oauth::new_google_client(
    &gdrivefs::get_contents(&args.flag_client_id_file).expect(&format!(
      "Error while getting content of file: {}",
      &args.flag_client_id_file
    )),
    &gdrivefs::get_contents(&args.flag_client_secret_file).expect(&format!(
      "Error while getting content of file: {}",
      &args.flag_client_secret_file
    )),
    None
  );

  let authenticator = oauth::GoogleAuthenticator::from_file(client, &args.flag_token_file).unwrap();
  authenticator.start_auto_save(&args.flag_token_file, std::time::Duration::new(60, 0));

  println!("Mounting drive fs at {:?}", args.arg_mountpoint);

  let options = gdrivefs::FileReadOptions {
    readahead_queue_size: args.flag_readahead_queue_size,
    file_read_cache_blocks: args.flag_file_read_cache_blocks,
    read_block_multiplier: args.flag_read_block_multiplier,
  };

  let driveimpl = gdrivefs::GDriveFS::new(authenticator, options);
  if args.flag_dir_poll_secs > 0 {
    driveimpl.start_auto_refresh(std::time::Duration::new(args.flag_dir_poll_secs as u64, 0));
  }

  // todo(jonallie): figure out how to make this the default using docopt.
  fuse::mount(
    driveimpl,
    &args.arg_mountpoint,
    &[std::ffi::OsStr::new("-oallow_other")],
  ).expect(&format!(
    "Could not mount fuse filesystem at {}",
    &args.arg_mountpoint
  ));
}
