FROM node:18-alpine

WORKDIR /app

# install taskfile
RUN sh -c "$(curl --location https://taskfile.dev/install.sh)" -- -d -b /usr/local/bin/
COPY Taskfile.yml /app

COPY package*.json /app
RUN npm install

COPY --chown=node:node server.js /app
ADD --chown=node:node models /app/models
ADD --chown=node:node public /app/public
CMD ["node", "server.js"]

