The relatedness scorer is four named scorers over one candidate set: shared words by
inverse document frequency, distance along declared edges, the relations between the
target and one other issue, and what the pair have in common. The six copies of the
same `entry().or_insert_with()` incantation are one `bump` call each.
