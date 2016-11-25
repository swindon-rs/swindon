from collections import deque, defaultdict


USERS = {}
ROOMS = {}


class Room(object):

    def __init__(self, name):
        self.name = name
        self.counter = 0
        self.messages = deque(max_length=64)

    def add(self, text):
        self.counter += 1
        self.messages.append({'id': self.counter, 'text': text})


class User(object):

    def __init__(self, uid, **props):
        self.uid = uid
        self.__dict__.update(props)
        self.rooms_last_seen = defaultdict(int)

    def update(self, meta):
        self.__dict__.update(meta)

    def initial_lattice(self):
        shared = {}
        mine = {}
        for k, n in self.rooms_last_seen.items():
            shared[k] = {'last_message_counter': self.rooms[k].counter}
            mine[k] = {'last_seen_counter': n}
        return {
            'shared': shared,
            'private': {
                self.uid: mine,
            }
        }


def ensure_user(uid, **meta):
    if uid not in USERS:
        user = USERS[uid] = User(uid, **meta)
    else:
        user = USERS[uid]
        user.update(meta)
    return user

