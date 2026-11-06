# fuzz

`cargo fuzz` targets for the readers, the round trip and the file name
parsers. The local corpus, crash artifacts, coverage and build output stay
out of the repository through `.gitignore`.

## Seed Corpus

`fuzz/seeds/<target>/` holds the minimised seed corpus of each target, grown
from the official samples, the `tests/corpus` invoices, the `tests/fixtures`
signed files and hand-picked file names, then reduced with
`-set_cover_merge=1`. `.clusterfuzzlite/build.sh` zips each corpus into
`$OUT/<target>_seed_corpus.zip`, which the ClusterFuzzLite runner unpacks as
the starting corpus, and the build fails a target whose directory is empty.
