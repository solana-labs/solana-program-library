declare module emailAddresses {
    function parseOneAddress(input: string | Options): ParsedMailbox | ParsedGroup | null;
    function parseAddressList(input: string | Options): (ParsedMailbox | ParsedGroup)[] | null;
    function parseFrom(input: string | Options): (ParsedMailbox | ParsedGroup)[] | null;
    function parseSender(input: string | Options): ParsedMailbox | ParsedGroup | null;
    function parseReplyTo(input: string | Options): (ParsedMailbox | ParsedGroup)[] | null;

    interface ParsedMailbox {
        node?: ASTNode;
        parts: {
            name: ASTNode | null;
            address: ASTNode;
            local: ASTNode;
            domain: ASTNode;
            comments: ASTNode[];
        };
        type: "mailbox";
        name: string | null;
        address: string;
        local: string;
        domain: string;
    }

    interface ParsedGroup {
        node?: ASTNode;
        parts: {
            name: ASTNode;
        };
        type: "group";
        name: string;
        addresses: ParsedMailbox[];
    }

    interface ASTNode {
        name: string;
        tokens: string;
        semantic: string;
        children: ASTNode[];
    }

    type StartProductions =
        "address"
        | "address-list"
        | "angle-addr"
        | "from"
        | "group"
        | "mailbox"
        | "mailbox-list"
        | "reply-to"
        | "sender";

    interface Options {
        input: string;
        oneResult?: boolean;
        partial?: boolean;
        rejectTLD?: boolean;
        rfc6532?: boolean;
        simple?: boolean;
        startAt?: StartProductions;
        strict?: boolean;
        atInDisplayName?: boolean;
        commaInDisplayName?: boolean;
        addressListSeparator?: string;
    }

    interface ParsedResult {
        ast: ASTNode;
        addresses: (ParsedMailbox | ParsedGroup)[];
    }
}

declare function emailAddresses(opts: emailAddresses.Options): emailAddresses.ParsedResult | null;

declare module "email-addresses" {
    export = emailAddresses;
}

/* Example usage:

// Run this file with:
//  tsc test.ts && NODE_PATH="../emailaddresses/lib" node test.js
/// <reference path="../emailaddresses/lib/email-addresses.d.ts"/>
import emailAddresses = require('email-addresses');

function isParsedMailbox(mailboxOrGroup: emailAddresses.ParsedMailbox | emailAddresses.ParsedGroup): mailboxOrGroup is emailAddresses.ParsedMailbox {
    return mailboxOrGroup.type === 'mailbox';
}

var testEmail : string = "TestName (a comment) <test@example.com>";
console.log(testEmail);

var parsed = emailAddresses.parseOneAddress(testEmail);
console.log(parsed);

var a : string = parsed.parts.name.children[0].name;
console.log(a);

if (isParsedMailbox(parsed)) {
    var comment : string = parsed.parts.comments[0].tokens;
    console.log(comment);
} else {
    console.error('error, should be a ParsedMailbox');
}

//

var emailList : string = "TestName <test@example.com>, TestName2 <test2@example.com>";
console.log(emailList);

var parsedList = emailAddresses.parseAddressList(emailList);
console.log(parsedList);

var b : string = parsedList[1].parts.name.children[0].semantic;
console.log(b);

//

var parsedByModuleFxn = emailAddresses({ input: emailList, rfc6532: true });
console.log(parsedByModuleFxn.addresses[0].name);

*/
