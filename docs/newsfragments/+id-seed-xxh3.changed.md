Id suffixes come from xxh3 over the project name, keyed by a seed, then a
one-at-a-time walk through the base-36 space. `VISSUE_ID_SEED` pins the seed and
makes a minting sequence reproducible.

The previous derivation was a single multiply by Knuth's constant over a
nanosecond clock, taking base-36 digits off the low bits and placing the next
probe 17 away from the last. Successive probes correlated, and two processes
reading the same coarse clock walked the same short arithmetic sequence. This
crate already depended on xxh3 for the export digest, so the better start cost no
new dependency.

The walk matters as much as the hash. A first version hashed each probe
independently, which draws with replacement and can miss a free suffix that
exists: with 1295 of 1296 taken, 2592 draws find the survivor about six times in
seven, and a test that had to find it failed one run in seven. Stepping visits
each suffix once.

Pinning the seed is what gives a racing-creates test power. With a clock seed two
racers draw from 36^4 and never collide by luck, so such a test passes whether or
not the id reservation is read under the lock; pinned, it fails on every id when
the reservation is stale.
