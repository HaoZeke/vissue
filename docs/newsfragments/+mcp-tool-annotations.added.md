Every MCP tool now declares whether it reads or writes, whether a write is
destructive, and whether calling it twice changes anything more than calling it
once. A client can tell `vissue_list` from `vissue_normalize` without calling
either, which it could not before: forty-six tools carried no hints at all.
