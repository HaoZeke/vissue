`vissue_list`, `vissue_ready`, `vissue_show` and `vissue_digest` return
`structuredContent` with an `outputSchema`, rather than a pretty-printed JSON
string inside a text block. The schema comes from the types themselves, so it
cannot drift from what is returned. The serialized text stays beside it, for a
client that reads only text.
