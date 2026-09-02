# Retained verification failures

## Gate rejection

`scripts/research_gate.py` was deliberately invoked with the closest retained Windows files. It rejected them before producing an output file because they do not match the immutable gate: 384 rather than 768 dimensions, 100,000 rather than 1,000,000 rows, 100,000 rather than 10,000 eligible rows, a 0.95 rather than 0.99 recall target, and different split ranges. This is an expected rejection, not a benchmark pass or failure.

## Test invocation correction

The initial command `python -m unittest scripts/test_compare_runs.py scripts/test_research_gate.py` failed during module import because those tests import sibling modules as top-level names. No test body ran. The repository-correct invocation `python -m unittest discover -s scripts -p 'test_*.py'` subsequently ran 13 tests successfully. This is a test-command error, not a product or benchmark failure.
