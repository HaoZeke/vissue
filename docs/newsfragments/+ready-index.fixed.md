`ready` no longer rescans the corpus once per issue to find that issue's parent
and siblings. It was quadratic in the corpus on any tracker whose issues have
parents, which is every tracker with a plan in it, and `ready` is the verb an
agent polls. On 20,000 issues across 10 projects the call drops from 617 ms to
512 ms, and the gap widens as parents sit further from the top of their file.
