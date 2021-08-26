import {ROUND_INIT_TIME} from "../src/main";

export function assert(condition: boolean, message?: string) {
    if (!condition) {
        console.log(Error().stack + ':main.ts');
        throw message || 'Assertion failed';
    }
}

export async function waitForRoundtoEnd() {
    //weird semantics - but needed to work inside jest
    //taken from https://stackoverflow.com/questions/46077176/jest-settimeout-not-pausing-test
    await new Promise(res => setTimeout(() => {
            res(0)
        }, ROUND_INIT_TIME + 3000) //empirically found 3s on top is enough
    );
}