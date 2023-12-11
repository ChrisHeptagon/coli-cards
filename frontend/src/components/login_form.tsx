
export function LoginForm({
    type,
}: {
    type: "login" | "register"
}
) {
    const header = type === "login" ? "Login" : "Register"
    const autocomplete = type === "login" ? "username" : "new-password"
    return (
    <form  method="post" id="login-form">
	<h1>{header}</h1>
	<div id="input">
		<label htmlFor="username">Username</label>
		<input type="text" id="username" autoComplete={autocomplete}/>
		<label htmlFor="password">Password</label>
		<input type="password" id="password" autoComplete={autocomplete}/>
	</div>
    <button type="submit">Login</button>
</form>
    )
}