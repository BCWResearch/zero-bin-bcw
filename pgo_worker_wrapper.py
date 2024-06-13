# expects GCS_UPLOAD_PATH environment variable
# optional LLVM_PROFILE_FILE environment variable
#   - if this one is not set, it will assume: `./target/pgo-profiles/`

import signal
import sys
import os
from google.cloud import storage


GCS_UPLOAD_BUCKET = os.environ.get("GCS_UPLOAD_BUCKET")
assert(GCS_UPLOAD_BUCKET is not None)

LLVM_PROFILE_DIRECTORY = os.environ.get("LLVM_PROFILE_DIRECTORY")
if LLVM_PROFILE_DIRECTORY is None:
    LLVM_PROFILE_DIRECTORY = "./target/pgo-profiles/"


def signal_handler(sig, frame):
    ''' for handling interrupts (currently SIGINT and SIGTERM) '''
    if sig == signal.SIGINT:
        print('Interrupted by user (SIGINT)')

    elif sig == signal.SIGTERM:
        print('Termination signal received (SIGTERM)')
    else:
        print("Received an unexpected signal. Ignoring...")
        return
    #assert(indexing_range_serialized is not None)
    #future = publisher.publish(topic=STATEFUL_RESUMPTION_TOPIC_PATH, data=indexing_range_serialized)
    #print(future.result())
    sys.exit(0)

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

# TODO: run the worker binary:




# check that there is exactly 1 .profraw file exists at the path specified in `LLVM_PROFILE_FILE`,
#   then upload it to GCS
files = os.listdir(LLVM_PROFILE_DIRECTORY)
if len(files) != 1:
    print("FATAL: more or less than 1 file in the profiling directory:", files)
    print("The profiling directory is:", LLVM_PROFILE_DIRECTORY)
    print("Exiting...")
    sys.exit(1)
else:
    pgo_file = files[0]
    if pgo_file.endswith(".profraw"):
        upload_pgo_file_to_gcs(pgo_file)
    else:
        print("FATAL: unexpected file type in the profiling directory:", pgo_file)
        print("The profiling directory is:", LLVM_PROFILE_DIRECTORY)
        print("Exiting...")
        sys.exit(1)
