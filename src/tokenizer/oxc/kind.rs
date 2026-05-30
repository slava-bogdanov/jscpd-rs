use oxc_parser::Kind;

use super::super::TokenKind;
use super::lexical::is_js_constant;

pub(super) fn oxc_token_kind(kind: Kind, value: &str) -> TokenKind {
    if kind == Kind::Ident && is_js_constant(value) {
        TokenKind::Constant
    } else {
        token_kind_for_oxc(kind)
    }
}

fn token_kind_for_oxc(kind: Kind) -> TokenKind {
    if kind.is_number() {
        return TokenKind::Number;
    }
    if matches!(
        kind,
        Kind::Str
            | Kind::NoSubstitutionTemplate
            | Kind::TemplateHead
            | Kind::TemplateMiddle
            | Kind::TemplateTail
            | Kind::RegExp
    ) {
        return TokenKind::String;
    }
    if is_oxc_keyword(kind) {
        return TokenKind::Keyword;
    }
    if is_oxc_punctuation(kind) {
        return TokenKind::Punctuation;
    }
    if is_oxc_operator(kind) {
        return TokenKind::Operator;
    }
    TokenKind::Default
}

fn is_oxc_keyword(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Await
            | Kind::Break
            | Kind::Case
            | Kind::Catch
            | Kind::Class
            | Kind::Const
            | Kind::Continue
            | Kind::Debugger
            | Kind::Default
            | Kind::Delete
            | Kind::Do
            | Kind::Else
            | Kind::Enum
            | Kind::Export
            | Kind::Extends
            | Kind::Finally
            | Kind::For
            | Kind::Function
            | Kind::If
            | Kind::Import
            | Kind::In
            | Kind::Instanceof
            | Kind::New
            | Kind::Return
            | Kind::Super
            | Kind::Switch
            | Kind::This
            | Kind::Throw
            | Kind::Try
            | Kind::Typeof
            | Kind::Var
            | Kind::Void
            | Kind::While
            | Kind::With
            | Kind::Async
            | Kind::From
            | Kind::Get
            | Kind::Of
            | Kind::Set
            | Kind::As
            | Kind::Type
            | Kind::Undefined
            | Kind::Implements
            | Kind::Interface
            | Kind::Let
            | Kind::Package
            | Kind::Private
            | Kind::Protected
            | Kind::Public
            | Kind::Static
            | Kind::Yield
            | Kind::True
            | Kind::False
            | Kind::Null
    )
}

fn is_oxc_punctuation(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Colon
            | Kind::Comma
            | Kind::Dot
            | Kind::Dot3
            | Kind::LBrack
            | Kind::LCurly
            | Kind::LParen
            | Kind::RBrack
            | Kind::RCurly
            | Kind::RParen
            | Kind::Semicolon
    )
}

fn is_oxc_operator(kind: Kind) -> bool {
    !matches!(kind, Kind::Ident | Kind::PrivateIdentifier | Kind::JSXText)
        && !matches!(token_kind_for_operator_check(kind), TokenKind::Default)
}

fn token_kind_for_operator_check(kind: Kind) -> TokenKind {
    if matches!(
        kind,
        Kind::Amp
            | Kind::Amp2
            | Kind::Amp2Eq
            | Kind::AmpEq
            | Kind::Bang
            | Kind::Caret
            | Kind::CaretEq
            | Kind::Eq
            | Kind::Eq2
            | Kind::Eq3
            | Kind::GtEq
            | Kind::LAngle
            | Kind::LtEq
            | Kind::Minus
            | Kind::Minus2
            | Kind::MinusEq
            | Kind::Neq
            | Kind::Neq2
            | Kind::Percent
            | Kind::PercentEq
            | Kind::Pipe
            | Kind::Pipe2
            | Kind::Pipe2Eq
            | Kind::PipeEq
            | Kind::Plus
            | Kind::Plus2
            | Kind::PlusEq
            | Kind::Question
            | Kind::Question2
            | Kind::Question2Eq
            | Kind::QuestionDot
            | Kind::RAngle
            | Kind::ShiftLeft
            | Kind::ShiftLeftEq
            | Kind::ShiftRight
            | Kind::ShiftRight3
            | Kind::ShiftRight3Eq
            | Kind::ShiftRightEq
            | Kind::Slash
            | Kind::SlashEq
            | Kind::Star
            | Kind::Star2
            | Kind::Star2Eq
            | Kind::StarEq
            | Kind::Tilde
            | Kind::Arrow
    ) {
        TokenKind::Operator
    } else {
        TokenKind::Default
    }
}
