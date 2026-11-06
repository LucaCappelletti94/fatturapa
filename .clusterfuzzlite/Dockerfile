FROM gcr.io/oss-fuzz-base/base-builder-rust
# The roundtrip fuzz target validates documents with xmllint; the run phases
# execute inside this image, so the tool and the schemas must both be here.
RUN apt-get update && apt-get install --yes --no-install-recommends libxml2-utils
COPY . $SRC/fatturapa
WORKDIR $SRC/fatturapa
COPY ./.clusterfuzzlite/build.sh $SRC/
