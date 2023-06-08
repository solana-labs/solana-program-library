import React from 'react'
import Logo from './Logo'
import { Input, InputLeftElement, InputGroup, InputRightElement, Avatar, Text } from '@chakra-ui/react'
import { BellIcon, SearchIcon, ChevronDownIcon } from '@chakra-ui/icons'
import Button from './Button'

export default function NavBar() {
    return (
        <header className='sticky top-0 left-0 right-0 bg-neutral-800 w-full p-4 text-neutral-400 flex justify-between'>
            <div className="flex gap-4 items-center">
                <div className="flex gap-2 items-center">
                    <Logo />
                    <InputGroup variant={'outline'} size={'sm'} className='w-fit text-neutral-500 bg-neutral-900 border-0'>
                        <InputLeftElement
                            pointerEvents='none'
                        >
                            <SearchIcon color="gray.100" />

                        </InputLeftElement>
                        <Input className='border border-neutral-500' placeholder='Organization' />
                        <InputRightElement
                            pointerEvents='none'
                            children="/"

                        />
                    </InputGroup>
                </div>
                <div >
                    <ul className="flex gap-4 items-center">
                        <li>My Feed</li>
                        <li>Ecosystem Feed</li>
                        <li>Discover</li>
                    </ul>
                </div>
            </div>
            <div className="flex items-center gap-3">
                <Button
                    text='+ Create Hub'
                    type='button'
                    variant='link'
                    rounded={true}
                />
                <div className='relative'>
                    <BellIcon color="gray.300" boxSize={8} />
                    <Text className='absolute top-0 right-0 p-1 text-white bg-red-600 rounded-full' fontSize='3xs'>9+</Text>
                </div>
                <div className='flex items-center'>
                    <Avatar name="John Doe" size={'sm'} className='mr-1' />
                    @buckybuddyy...
                    <ChevronDownIcon />
                </div>
            </div>
        </header>
    )
}
