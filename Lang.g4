grammar Lang;

// -----------------------------------------------------------------------------
// Parser rules
// -----------------------------------------------------------------------------

program
    : separators? (statement (separators statement)*)? separators? EOF
    ;

statement
    : PUB* statementCore
    ;

statementCore
    : useDecl
    | varDecl
    | returnStmt
    | forStmt
    | ifExpr
    | whenExpr
    | dataDecl
    | functionDecl
    | immutableDecl
    | commandCallStmt
    | assignmentStmt
    | expressionStmt
    ;

useDecl
    : USE identifierName (DOT identifierName)*
    ;

varDecl
    : VAR identifierName (COLON typeAnnotation)? ASSIGN expression
    ;

// The hand-written parser treats a bare "name = value" at statement position
// as the immutable-declaration form. Reassignment validity is resolved later.
immutableDecl
    : identifierName ASSIGN expression
    ;

returnStmt
    : RETURN expression?
    ;

forStmt
    : FOR identifierName IN expression block
    ;

ifExpr
    : IF expression block (NEWLINE* ELSE (ifExpr | block))?
    ;

block
    : LBRACE separators? (statement (separators statement)*)? separators? RBRACE
    ;

functionDecl
    : IDENT parameterClause functionBody
    ;

functionBody
    : FATARROW expression
    | block
    ;

// The Go parser classifies a declaration as data when its name starts with an
// uppercase Unicode letter. A body is enough to make it a declaration. Without
// a body, at least one top-level parameter must be typed; otherwise Foo(...) is
// parsed as an ordinary call expression.
dataDecl
    : DATA_IDENT LPAREN softSeparator* paramList? softSeparator* RPAREN dataBody
    | DATA_IDENT LPAREN softSeparator* paramsWithType softSeparator* RPAREN
    ;

dataBody
    : LBRACE separators? (computedField (separators computedField)*)? separators? RBRACE
    ;

computedField
    : identifierName (ASSIGN | FATARROW) expression
    ;

parameterClause
    : LPAREN softSeparator* paramList? softSeparator* RPAREN
    ;

paramList
    : param (commaSep param)*
    ;

// Requires at least one typed parameter somewhere in the list.
paramsWithType
    : typedParam (commaSep param)*
    | untypedParam commaSep paramsWithType
    ;

param
    : identifierName (COLON typeAnnotation)?
    ;

typedParam
    : identifierName COLON typeAnnotation
    ;

untypedParam
    : identifierName
    ;

typeAnnotation
    : identifierName QUESTION?
    ;

commaSep
    : softSeparator* COMMA softSeparator*
    ;

// Command-style calls are statement-only in the reference parser. The first
// argument must begin immediately after the callee token (no newline/';') and
// must start with one of the token kinds accepted by isCommandStyleCall().
// Examples: print "Hello", print user.name, print -1.
commandCallStmt
    : identifierName commandArgument (COMMA expression)*
    ;

// Same precedence as expression, but the *first* primary may not be parenthesized.
// This prevents `print (1)` from being classified as command style; the reference
// parser treats that spelling as the ordinary call expression `print(1)`.
commandArgument
    : commandElvisExpression
    ;

commandElvisExpression
    : commandOrExpression (ELVIS orExpression)*
    ;

commandOrExpression
    : commandAndExpression (OR andExpression)*
    ;

commandAndExpression
    : commandEqualityExpression (AND equalityExpression)*
    ;

commandEqualityExpression
    : commandComparisonExpression ((EQ | NEQ) comparisonExpression)*
    ;

commandComparisonExpression
    : commandAdditiveExpression ((LT | LTE | GT | GTE) additiveExpression)*
    ;

commandAdditiveExpression
    : commandMultiplicativeExpression ((PLUS | MINUS) multiplicativeExpression)*
    ;

commandMultiplicativeExpression
    : commandUnaryExpression ((STAR | SLASH | PERCENT) unaryExpression)*
    ;

commandUnaryExpression
    : (MINUS | BANG) unaryExpression
    | commandPostfixExpression
    ;

commandPostfixExpression
    : commandPrimaryExpression postfixSuffix*
    ;

commandPrimaryExpression
    : INT
    | FLOAT
    | STRING
    | TRUE
    | FALSE
    | NULL
    | identifierName
    | listLiteral
    | ifExpr
    | whenExpr
    ;

// Assignment is deliberately syntactic here; the reference parser also accepts
// broad expression targets and lets later stages reject invalid l-values.
assignmentStmt
    : expression ASSIGN expression
    ;

expressionStmt
    : expression
    ;

whenExpr
    : WHEN expression? LBRACE separators?
      (whenCase (separators whenCase)*)?
      separators? RBRACE
    ;

whenCase
    : ELSE FATARROW whenBody
    | patternList (IF expression)? FATARROW whenBody
    ;

patternList
    : expression (COMMA softSeparator* expression)*
    ;

whenBody
    : block
    | expression
    ;

expression
    : elvisExpression
    ;

elvisExpression
    : orExpression (ELVIS orExpression)*
    ;

orExpression
    : andExpression (OR andExpression)*
    ;

andExpression
    : equalityExpression (AND equalityExpression)*
    ;

equalityExpression
    : comparisonExpression ((EQ | NEQ) comparisonExpression)*
    ;

comparisonExpression
    : additiveExpression ((LT | LTE | GT | GTE) additiveExpression)*
    ;

additiveExpression
    : multiplicativeExpression ((PLUS | MINUS) multiplicativeExpression)*
    ;

multiplicativeExpression
    : unaryExpression ((STAR | SLASH | PERCENT) unaryExpression)*
    ;

unaryExpression
    : (MINUS | BANG) unaryExpression
    | postfixExpression
    ;

postfixExpression
    : primaryExpression postfixSuffix*
    ;

postfixSuffix
    : callSuffix
    | memberSuffix
    | safeMemberSuffix
    | indexSuffix
    ;

callSuffix
    : LPAREN softSeparator* argumentList? softSeparator* RPAREN
    ;

argumentList
    : expression (commaSep expression)*
    ;

memberSuffix
    : DOT identifierName
    ;

safeMemberSuffix
    : QDOT identifierName
    ;

indexSuffix
    : LBRACK expression RBRACK
    ;

primaryExpression
    : INT
    | FLOAT
    | STRING
    | TRUE
    | FALSE
    | NULL
    | identifierName
    | LPAREN softSeparator* expression softSeparator* RPAREN
    | listLiteral
    | ifExpr
    | whenExpr
    ;

listLiteral
    : LBRACK softSeparator*
      (expression (commaSep expression)*)?
      softSeparator* RBRACK
    ;

identifierName
    : IDENT
    | DATA_IDENT
    ;

separators
    : softSeparator+
    ;

softSeparator
    : NEWLINE
    | SEMI
    ;

// -----------------------------------------------------------------------------
// Lexer rules
// -----------------------------------------------------------------------------

TRUE    : 'true';
FALSE   : 'false';
NULL    : 'null';
USE     : 'use';
PUB     : 'pub';
VAR     : 'var';
IF      : 'if';
ELSE    : 'else';
FOR     : 'for';
IN      : 'in';
WHEN    : 'when';
RETURN  : 'return';
OBJECT  : 'object';
EXTEND  : 'extend';
SHAPE   : 'shape';
ASYNC   : 'async';
AWAIT   : 'await';
TRY     : 'try';
THROW   : 'throw';

FATARROW : '=>';
ARROW    : '->';
EQ       : '==';
NEQ      : '!=';
LTE      : '<=';
GTE      : '>=';
AND      : '&&';
OR       : '||';
QDOT     : '?.';
ELVIS    : '?:';
ASSIGN   : '=';
PLUS     : '+';
MINUS    : '-';
STAR     : '*';
SLASH    : '/';
PERCENT  : '%';
LT       : '<';
GT       : '>';
BANG     : '!';
DOT      : '.';
COMMA    : ',';
COLON    : ':';
SEMI     : ';';
QUESTION : '?';
LPAREN   : '(';
RPAREN   : ')';
LBRACE   : '{';
RBRACE   : '}';
LBRACK   : '[';
RBRACK   : ']';

FLOAT
    : DIGIT+ '.' DIGIT+
    ;

INT
    : DIGIT+
    ;

// The reference lexer accepts any escaped character. It decodes common escapes
// (n, t, r, \\, \", $, 0) after lexing; generated ANTLR token text remains raw.
STRING
    : '"' (ESCAPE | ~["\\\n])* '"'
    ;

// Put DATA_IDENT before IDENT. Both can match uppercase-starting names, and
// ANTLR resolves equal-length matches by rule order.
DATA_IDENT
    : [\p{Lu}] [\p{L}\p{Nd}_]*
    ;

IDENT
    : [\p{L}_] [\p{L}\p{Nd}_]*
    ;

NEWLINE
    : '\n'
    ;

LINE_COMMENT
    : '//' ~[\n]* -> skip
    ;

WS
    : [ \t\r]+ -> skip
    ;

fragment ESCAPE
    : '\\' .
    ;

fragment DIGIT
    : [0-9]
    ;
