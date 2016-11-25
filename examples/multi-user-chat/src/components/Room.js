import React, { Component } from 'react'
import classnames from 'classnames'
import * as websocket from '../websocket'

export class Title extends Component {
  render() {
    const { params: {roomName} } = this.props;
    return <h1 className="room-title">{ roomName }</h1>
  }
}

export class Main extends Component {
  render() {
    const { className } = this.props;
    return (
      <div className={ classnames("Room", className) }>
        <div className="message-box">
          <ul className="messages">
            { (websocket.current_room_messages || []).map(m => (
                <li key={m.id}>
                  <span className="message--author">{ m.author }</span>
                  <span className="message--text">{ m.text }</span>
                </li>
              ))
            }
          </ul>
        </div>
        <div className="inputbox">
          <input type="text" placeholder="Your message"
                value={(this.state && this.state.text) || ''}
                onChange={ e => this.setState({text: e.target.value}) }
                onKeyDown={ e => {
                    if(e.which === 13 && this.state && this.state.text) {
                      websocket.send_message(this.state.text)
                      this.setState({text: ''})
                    }
                }}/>
        </div>
      </div>
    )
  }
}
