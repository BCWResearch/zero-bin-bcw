# expects GCS_UPLOAD_PATH environment variable
# optional WORKER_RELATIVE_PATH environment variable
#   - if this one is not set, it will assume: `./target/x86_64-unknown-linux-gnu/release/worker`
# optional PROFILE_DIRECTORY environment variable
#   - if this one is not set, it will assume: `./target/pgo-profiles/`

import signal
import subprocess
import sys
import os
import time
from google.cloud import storage


GCS_UPLOAD_BUCKET = os.environ.get("GCS_UPLOAD_BUCKET")
assert(GCS_UPLOAD_BUCKET is not None)

WORKER_PATH = os.environ.get("WORKER_PATH")
if WORKER_PATH is None:
    WORKER_PATH = "./target/x86_64-unknown-linux-gnu/release/worker"

PROFILE_DIRECTORY = os.environ.get("PROFILE_DIRECTORY")
if PROFILE_DIRECTORY is None:
    PROFILE_DIRECTORY = "./target/pgo-profiles/"


def signal_handler(sig, frame):
    ''' for handling interrupts (currently SIGINT and SIGTERM) '''
    if sig == signal.SIGINT:
        print('Interrupted by user (SIGINT)')
        process.send_signal(signal.SIGINT)
        cleanup_pgo_run()
    elif sig == signal.SIGTERM:
        print('Termination signal received (SIGTERM)')
        process.send_signal(signal.SIGTERM)
        cleanup_pgo_run()
    else:
        print("Ignoring an unexpected signal:", sig)
        return

# -------------------------
# | run the worker binary |
# -------------------------
process = subprocess.Popen(WORKER_PATH)

# register the signal handlers
signal.signal(signal.SIGINT, signal_handler)
signal.signal(signal.SIGTERM, signal_handler)


# upload the pgo file to the gcs bucket
def upload_pgo_file_to_gcs(file_path):
    storage_client = storage.Client()

    # Get the bucket
    bucket = storage_client.bucket(GCS_UPLOAD_BUCKET)

    # Create a new blob and upload the file's content
    file_name = os.path.basename(file_path)
    blob = bucket.blob(file_name)
    blob.upload_from_filename(file_path)

def cleanup_pgo_run():
    # continually checks if the PGO file has been generated before uploading to GCS
    files = os.listdir(PROFILE_DIRECTORY)
    while len(files) < 1:
        time.sleep(1)
        files = os.listdir(PROFILE_DIRECTORY)

    if len(files) > 1:
        print("FATAL: more than 1 file in the profiling directory:", files)
        print("The profiling directory is:", PROFILE_DIRECTORY)
        print("Exiting...")
        sys.exit(1)
    else:
        pgo_file = files[0]
        if pgo_file.endswith(".profraw"):
            upload_pgo_file_to_gcs(pgo_file)
        else:
            print("FATAL: unexpected file extension (should be .profraw) in the profiling directory. File is:", pgo_file)
            print("The profiling directory is:", PROFILE_DIRECTORY)
            print("Exiting...")
            sys.exit(1)
