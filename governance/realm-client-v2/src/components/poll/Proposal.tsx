import React from 'react'

export default function Proposal() {
    return (
        <div className='w-fit p-4 mx-auto my-2 '>
            <div className="flex items-center gap-2 text-sm ">
                <p className='bg-black p-2 text-white rounded'>
                    DAO Name
                </p>
                <p className='bg-neutral-800 text-neutral-500 p-2 rounded'>
                    Org type: <span className='text-neutral-50'>
                        Community Token DAO
                    </span>
                </p>
            </div>
            <h1 className='text-4xl font-bold text-white'>
                Let's create a proposal
            </h1>

            <div className="bg-black">
                <small className='capitalize'>Proposal rules</small>
                <hr />
                <p>
                    Which wallet's rules should this proposal follow?
                </p>
                <small>
                    These rules determin voting duration, voting threshold, and vote tipping.
                </small>
                <div className="bg-neutral-800">
                <div>
                    wallet address
                    $1200
                </div>
                <div className="">
                    <p>
                        Wallet address
                    </p>
                </div>
                </div>
            </div>

        </div>
    )
}
