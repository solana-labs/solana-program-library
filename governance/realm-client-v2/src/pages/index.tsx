import NavBar from '@/components/common/NavBar'
import Steps from '@/components/common/Steps'
import Proposal from '@/components/poll/Proposal'


export default function Home() {
  return (
    <main className=' w-full h-screen'>
      <NavBar />
      <Steps/>
      <Proposal/>
    </main>
  )
}
