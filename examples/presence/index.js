import cookie from 'cookie'
import auth from './auth.marko'
import user_list from './user_list.marko'
import { Router } from 'marko-path-router'

var {swindon_presence_login} = cookie.parse(document.cookie)

let render = Router.renderSync({
    initialRoute: swindon_presence_login ? '/list' : '/login',
    routes: [
        {path: '/login', component: auth },
        {path: '/list', component: user_list },
    ],
})

render.appendTo(document.body)
