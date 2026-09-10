`vissue satchel` packs a slice of the tracker so somebody else can open it:
the issues named, everything they stand on, and the deed accessions their work
produced. The shape is BagIt, so a receiver checks a manifest before reading
anything, and the manifest has to account for the whole payload rather than
only for what it lists. `--seal` re-manifests after the deed store has filled
in the deeds; `--verify` is the receiver's check.
