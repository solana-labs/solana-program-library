import { Inter } from 'next/font/google'
import NavBar from '@/components/common/NavBar'

const inter = Inter({ subsets: ['latin'] })

export default function Home() {
  return (
    <main className=' w-full h-screen'>
      <NavBar />
      <p className='bg-black text-neutral-200 p-4'>
        Hello world
      </p>
    </main>
  )
}
