// src/routes.js
import React from 'react'
import { Router, Route, IndexRoute } from 'react-router'

import * as websocket from './websocket'
import Login from './components/Login'
import Chat from './components/Chat'
import * as room from './components/Room'
import * as select from './components/SelectRoom'
import {get_login} from './login'


function check_login(route, replace) {
  let {location: {pathname}} = route;
  if(pathname !== '/login' && !get_login()) {
    replace('/login')
  }
}

const Routes = ({history, ...props}) => (
  <Router history={history}>
    <Route path="/" onEnter={check_login}>
      <Route path="/login" component={Login} />
      <Route component={Chat}
            onEnter={websocket.start} onLeave={websocket.stop}>
        <IndexRoute component={select.Main} />
        <Route path=":roomName"
                    component={room.Main}
                    components={{ title: room.Title }} />
      </Route>
    </Route>
  </Router>
)

export default Routes
