/**
 * ironmark — C FFI bindings
 *
 * Compile the Rust crate as a static library first:
 *
 *   cargo build --release
 *
 * Then link against it:
 *
 *   cc -o example example.c -L target/release -l ironmark -lpthread -ldl
 *   (macOS: add -framework CoreFoundation -framework Security)
 *
 * Basic usage:
 *
 *   char *html = ironmark_render_html("# Hello\n");
 *   if (html) {
 *       printf("%s\n", html);
 *       ironmark_free(html);
 *   }
 *
 * With options (e.g. for untrusted input):
 *
 *   ironmark_options opts = ironmark_options_default();
 *   opts.flags |= IRONMARK_OPT_DISABLE_RAW_HTML;
 *   char *html = ironmark_render_html_with_options(buf, buf_len, &opts);
 */

#ifndef IRONMARK_H
#define IRONMARK_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Flag bits for ironmark_options.flags. Values are stable across releases. */
#define IRONMARK_OPT_HARD_BREAKS (UINT64_C(1) << 0)            /* newline in paragraph -> <br /> */
#define IRONMARK_OPT_HIGHLIGHT (UINT64_C(1) << 1)              /* ==text== -> <mark> */
#define IRONMARK_OPT_STRIKETHROUGH (UINT64_C(1) << 2)          /* ~~text~~ -> <del> */
#define IRONMARK_OPT_UNDERLINE (UINT64_C(1) << 3)              /* ++text++ -> <u> */
#define IRONMARK_OPT_TABLES (UINT64_C(1) << 4)                 /* pipe tables */
#define IRONMARK_OPT_AUTOLINK (UINT64_C(1) << 5)               /* bare URLs and emails */
#define IRONMARK_OPT_TASK_LISTS (UINT64_C(1) << 6)             /* - [ ] / - [x] */
#define IRONMARK_OPT_DISABLE_RAW_HTML (UINT64_C(1) << 7)       /* escape all raw HTML */
#define IRONMARK_OPT_HEADING_IDS (UINT64_C(1) << 8)            /* id= on headings */
#define IRONMARK_OPT_HEADING_ANCHORS (UINT64_C(1) << 9)        /* <a class="anchor"> in headings */
#define IRONMARK_OPT_INDENTED_CODE_BLOCKS (UINT64_C(1) << 10)  /* 4-space indented code */
#define IRONMARK_OPT_NO_HTML_BLOCKS (UINT64_C(1) << 11)        /* escape HTML blocks only */
#define IRONMARK_OPT_NO_HTML_SPANS (UINT64_C(1) << 12)         /* escape inline HTML only */
#define IRONMARK_OPT_TAG_FILTER (UINT64_C(1) << 13)            /* GFM tag filter */
#define IRONMARK_OPT_COLLAPSE_WHITESPACE (UINT64_C(1) << 14)   /* collapse spaces/tabs in text */
#define IRONMARK_OPT_PERMISSIVE_ATX_HEADERS (UINT64_C(1) << 15) /* #Heading without space */
#define IRONMARK_OPT_WIKI_LINKS (UINT64_C(1) << 16)            /* [[wiki link]] */
#define IRONMARK_OPT_LATEX_MATH (UINT64_C(1) << 17)            /* $math$ and $$math$$ */
#define IRONMARK_OPT_MDX (UINT64_C(1) << 18)                   /* MDX; trusted authors only */
#define IRONMARK_OPT_FRONTMATTER (UINT64_C(1) << 19)           /* skip leading front matter */

/* Every flag bit this version understands. */
#define IRONMARK_OPT_ALL ((UINT64_C(1) << 20) - 1)

/**
 * ironmark_options — parse options for ironmark_render_html_with_options.
 *
 * Always start from ironmark_options_default() and then set or clear bits,
 * so options added later keep their default values.
 */
typedef struct ironmark_options {
    uint64_t flags;           /* bitwise OR of IRONMARK_OPT_* */
    size_t max_nesting_depth; /* max blockquote/list nesting (default 128) */
    size_t max_input_size;    /* truncate input to this many bytes; 0 = unlimited */
} ironmark_options;

/**
 * ironmark_options_default — the options ironmark_render_html uses:
 * hard breaks, highlight, strikethrough, underline, tables, autolink,
 * task lists and indented code blocks on; everything else off;
 * max_nesting_depth 128; max_input_size 0 (unlimited).
 */
ironmark_options ironmark_options_default(void);

/**
 * ironmark_render_html — parse Markdown and return an HTML string.
 *
 * @param input  Null-terminated UTF-8 Markdown string.
 * @return       Heap-allocated, null-terminated HTML string.
 *               The caller MUST free this pointer with ironmark_free().
 *               Returns NULL if input is NULL or contains invalid UTF-8.
 *
 * Uses ironmark_options_default(). Raw HTML is passed through; for
 * untrusted input use ironmark_render_html_with_options() with
 * IRONMARK_OPT_DISABLE_RAW_HTML.
 *
 * Dangerous URI schemes (javascript:, vbscript:, data: except data:image/…)
 * are always stripped regardless of options.
 */
char *ironmark_render_html(const char *input);

/**
 * ironmark_render_html_with_options — parse Markdown with options.
 *
 * @param input    UTF-8 Markdown, len bytes. No terminating NUL needed.
 * @param len      Length of input in bytes.
 * @param options  Options, or NULL for ironmark_options_default().
 * @return         Heap-allocated, null-terminated HTML string.
 *                 The caller MUST free this pointer with ironmark_free().
 *                 Returns NULL if input is NULL, input is not valid UTF-8,
 *                 or options->flags has bits outside IRONMARK_OPT_ALL.
 */
char *ironmark_render_html_with_options(const char *input, size_t len,
                                        const ironmark_options *options);

/**
 * ironmark_free — free a string returned by ironmark_render_html or
 * ironmark_render_html_with_options.
 *
 * @param ptr  Pointer returned by one of those functions, or NULL (no-op).
 *
 * Passing any other pointer is undefined behaviour.
 */
void ironmark_free(char *ptr);

#ifdef __cplusplus
}
#endif

#endif /* IRONMARK_H */
