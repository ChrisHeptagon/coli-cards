import "@/app/main_layout.scss"
import Link from "next/link"

export default function Layout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <html lang="en">
      <body>
          <nav id="top_nav">
            <Link href="/">
            <h1>Coli-Cards</h1>
            </Link>
            <ul>
              <li>
                <Link href="/">Home</Link>
              </li>
              <li>
                <Link href="/flashcards">Flashcards</Link>
              </li>
              <li>
                <Link href="/about">About</Link>
              </li>
              <li>
                <Link href="/login">Login</Link>
              </li>
              <li>
                <Link href="/register">Register</Link>
              </li>
            </ul>
          </nav>
          <main id="main_content">
          {children}
          </main>
          <footer id="footer">
            <p>Coli-Cards &copy; {new Date().getFullYear()}</p>
          </footer>
        </body>
    </html>
  )
}
