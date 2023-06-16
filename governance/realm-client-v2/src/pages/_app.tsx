import '@/styles/globals.css'
import type { AppProps } from 'next/app'
import { ChakraProvider, extendTheme } from "@chakra-ui/react"
import { Inter } from 'next/font/google'


const inter = Inter({ subsets: ['latin'] })

const theme = extendTheme({
  colors: {
    brand: {
      50: "#97D3EF",
      500: "#0EA5E9",
      600: "#074865",
    },
  },
});

export default function App({ Component, pageProps }: AppProps) {
  return (
    <div className={`bg-neutral-900 ${inter.className} min-h-screen`}>
      <ChakraProvider theme={theme}>
        <Component {...pageProps} />
      </ChakraProvider>
    </div>
  )
}
