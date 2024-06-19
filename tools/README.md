# run-benchmark.sh
### Prerequisites
1. Write access to the zero-bin-bcw repo

### Steps
1. Go to https://github.com/BCWResearch/zero-bin-bcw/actions/workflows/benchmark.yml
1. Click Run workflow on the right of the page
1. Input the parameters needed for the test then click `Run Workflow`
1. Wait for your test run to appear, then click on it to view logs of the script.


### Tips
- View the logs of zero-bin-leader to get a better idea of the current progress. (A link will be printed in the script)
- If you expect a fast test run and the pipeline doesn't finish fast enough, check the zero-bin logs. If there is an error, you can cancel the pipeline and debug.
- Set the `block_start` and `block_end` to the same value to test only 1 block.
- By default, always set the same values for CPU requests/limits and memory requests/limits.
