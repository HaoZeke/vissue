A sentence in the explanation page had wrapped so that a number landed in column
zero, which Org reads as an ordered list item: the paragraph rendered split in
two around a stray list. `docs/scripts/check_org.py` now runs before the export
and refuses that shape, along with a citation whose reference has no entry, an
entry nobody cites, and a gap or a duplicate in the numbering. Neither class
produces invalid output, so nothing downstream was ever going to complain.
