//! Compile-time helpers for Molt's public command macros.

use std::collections::HashSet;

use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{
    bracketed,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    token::Comma,
    Error, LitStr, Result, Token,
};
use unicode_width::UnicodeWidthStr;

struct HelpEntry {
    name: LitStr,
    help: LitStr,
}

impl Parse for HelpEntry {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let content;
        syn::parenthesized!(content in input);
        let name = content.parse()?;
        content.parse::<Token![,]>()?;
        let help = content.parse()?;
        if !content.is_empty() {
            return Err(content.error("expected exactly (name, help)"));
        }
        Ok(Self { name, help })
    }
}

struct HelpEntries(Punctuated<HelpEntry, Comma>);

impl Parse for HelpEntries {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let content;
        bracketed!(content in input);
        let entries = content.parse_terminated(HelpEntry::parse, Token![,])?;
        if !input.is_empty() {
            return Err(input.error("unexpected tokens after help entries"));
        }
        Ok(Self(entries))
    }
}

fn format_help(entries: HelpEntries, reserved: (&str, &str)) -> Result<LitStr> {
    let mut names = HashSet::with_capacity(entries.0.len() + 1);
    names.insert(reserved.0.to_owned());

    for entry in &entries.0 {
        let name = entry.name.value();
        if !names.insert(name.clone()) {
            let message = if name == reserved.0 {
                format!("command name {name:?} is reserved")
            } else {
                format!("duplicate command name {name:?}")
            };
            return Err(Error::new(entry.name.span(), message));
        }
    }

    let width = entries
        .0
        .iter()
        .map(|entry| UnicodeWidthStr::width(entry.name.value().as_str()))
        .chain([UnicodeWidthStr::width(reserved.0)])
        .max()
        .unwrap_or_default();

    let mut output = String::new();
    for entry in &entries.0 {
        let name = entry.name.value();
        output.push_str("  ");
        output.push_str(&name);
        let help = entry.help.value();
        if !help.is_empty() {
            let padding = width.saturating_sub(UnicodeWidthStr::width(name.as_str()));
            output.extend(std::iter::repeat_n(' ', padding + 2));
            output.push_str(&help);
        }
        output.push('\n');
    }

    output.push_str("  ");
    output.push_str(reserved.0);
    if !reserved.1.is_empty() {
        output.extend(std::iter::repeat_n(
            ' ',
            width.saturating_sub(UnicodeWidthStr::width(reserved.0)) + 2,
        ));
        output.push_str(reserved.1);
    }

    let span = entries
        .0
        .first()
        .map_or_else(proc_macro2::Span::call_site, |entry| entry.name.span());
    Ok(LitStr::new(&output, span))
}

#[doc(hidden)]
#[proc_macro]
pub fn format_subcommand_help(input: TokenStream) -> TokenStream {
    let entries = parse_macro_input!(input as HelpEntries);
    match format_help(entries, ("-help", "")) {
        Ok(output) => output.into_token_stream().into(),
        Err(error) => error.into_compile_error().into(),
    }
}

#[doc(hidden)]
#[proc_macro]
pub fn format_command_help(input: TokenStream) -> TokenStream {
    let entries = parse_macro_input!(input as HelpEntries);
    match format_help(entries, ("help", "[-all]")) {
        Ok(output) => output.into_token_stream().into(),
        Err(error) => error.into_compile_error().into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    fn entries(tokens: proc_macro2::TokenStream) -> HelpEntries {
        syn::parse2(tokens).expect("valid help entries")
    }

    #[test]
    fn formats_empty_and_trailing_comma() {
        let help = format_help(entries(quote!([])), ("-help", "")).unwrap();
        assert_eq!(help.value(), "  -help");

        let help = format_help(
            entries(quote!([("one", "first"), ("two", "second"),])),
            ("-help", ""),
        )
        .unwrap();
        assert_eq!(help.value(), "  one    first\n  two    second\n  -help");

        let help = format_help(entries(quote!([("quiet", "")])), ("-help", "")).unwrap();
        assert_eq!(help.value(), "  quiet\n  -help");
    }

    #[test]
    fn aligns_by_unicode_display_width() {
        let help = format_help(
            entries(quote!([("短", "wide"), ("abc", "ascii")])),
            ("-help", ""),
        )
        .unwrap();
        assert_eq!(help.value(), "  短     wide\n  abc    ascii\n  -help");
    }

    #[test]
    fn rejects_duplicate_and_reserved_names() {
        let duplicate = format_help(
            entries(quote!([("same", "one"), ("same", "two")])),
            ("-help", ""),
        )
        .err()
        .unwrap();
        assert!(duplicate.to_string().contains("duplicate command name"));

        let reserved =
            format_help(entries(quote!([("-help", "collision")])), ("-help", ""))
                .err()
                .unwrap();
        assert!(reserved.to_string().contains("is reserved"));
    }

    #[test]
    fn requires_string_literals_and_rejects_old_shape() {
        assert!(syn::parse2::<HelpEntries>(quote!([(name, "help")])).is_err());
        assert!(syn::parse2::<HelpEntries>(quote!([("name", handler, "help")])).is_err());
    }
}
