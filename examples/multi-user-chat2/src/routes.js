// src/routes.js
import React from 'react'
import { Router, Route, IndexRoute } from 'react-router'

import * as server from './server'
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
            onEnter={server.start} onLeave={server.stop}>
        <IndexRoute component={select.Main} />
        <Route path=":roomName"
                    onEnter={server.enter_room}
                    onLeave={server.leave_room}
                    components={{ title: room.Title, children: room.Main }} />
      </Route>
    </Route>
  </Router>
)

export default Routes
