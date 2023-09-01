"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.memberReference = void 0;
const utils_1 = require("../../../../utils");
const memberReference = ({ urlTo }, props) => {
    const referenced = props.tryGetTargetReflectionDeep();
    if (!referenced) {
        return utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
            "Re-exports ",
            props.name);
    }
    if (props.name === referenced.name) {
        return (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
            "Re-exports ",
            utils_1.JSX.createElement("a", { href: urlTo(referenced) }, referenced.name)));
    }
    return (utils_1.JSX.createElement(utils_1.JSX.Fragment, null,
        "Renames and re-exports ",
        utils_1.JSX.createElement("a", { href: urlTo(referenced) }, referenced.name)));
};
exports.memberReference = memberReference;
