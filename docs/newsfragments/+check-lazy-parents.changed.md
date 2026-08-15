`check` only searches the wider tree for `:PARENT:` ids that are not
issues. It validates parents against any Org id, which includes design
documents and notes, so on a tracker sharing a root with a notes vault it
read every `.org` file to answer a question the issues had usually
already answered. Where every parent is another issue the scan is now
skipped entirely: 58ms to 23ms on a 35MB corpus. A tracker that does
point at a design document still pays for the search, which now stops as
soon as the ids it wants have been found.
