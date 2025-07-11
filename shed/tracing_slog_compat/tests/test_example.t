  $ $EXAMPLE slog
  T* [tracing_slog_compat_example] *main.rs:59] [tracing-span{bin: example}] Example tracing::trace (glob)
  D* [tracing_slog_compat_example] *main.rs:60] [tracing-span{bin: example}] number: 1, word: bird (glob)
  I* [tracing_slog_compat_example] *main.rs:61] [tracing-span{bin: example}] Example tracing::info, duration: 20ms (glob)
  W* [tracing_slog_compat_example] *main.rs:62] [tracing-span{bin: example}] Example tracing::warn (glob)
  E* [tracing_slog_compat_example] *main.rs:63] [tracing-span{bin: example}] Example tracing::error (200 300), var1: 100, word: cat (glob)
  E* [main] *main.rs:65] Example slog::error (glob)
  W* [main] *main.rs:66] Example slog::warn, display: value, debug: "value", key: value (glob)
  I* [main] *main.rs:67] Example slog::info 200 300, var1: 100 (glob)
  V* [main] *main.rs:68] Example slog::debug, duration: 20ms (glob)
  V* [main] *main.rs:69] Example slog::trace, string: example (glob)

  $ $EXAMPLE tracing
  T* [tracing_slog_compat_example] *main.rs:59] [tracing-span{bin: example}] Example tracing::trace (glob)
  D* [tracing_slog_compat_example] *main.rs:60] [tracing-span{bin: example}] number: 1, word: bird (glob)
  I* [tracing_slog_compat_example] *main.rs:61] [tracing-span{bin: example}] Example tracing::info, duration: 20ms (glob)
  W* [tracing_slog_compat_example] *main.rs:62] [tracing-span{bin: example}] Example tracing::warn (glob)
  E* [tracing_slog_compat_example] *main.rs:63] [tracing-span{bin: example}] Example tracing::error (200 300), var1: 100, word: cat (glob)
  E* [tracing_slog_compat_example] *main.rs:65] [tracing-span{bin: example}] Example slog::error (glob)
  W* [tracing_slog_compat_example] *main.rs:66] [tracing-span{bin: example}] Example slog::warn, key: "value", debug: "value", display: value (glob)
  I* [tracing_slog_compat_example] *main.rs:67] [tracing-span{bin: example}] Example slog::info 200 300, var1: 100 (glob)
  D* [tracing_slog_compat_example] *main.rs:68] [tracing-span{bin: example}] Example slog::debug, duration: 20ms (glob)
  T* [tracing_slog_compat_example] *main.rs:69] [tracing-span{bin: example}] Example slog::trace, string: example (glob)
