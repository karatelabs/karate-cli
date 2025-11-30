Feature: Smoke test for Karate CLI installer

Scenario: Fetch users from JSONPlaceholder API
* url 'https://jsonplaceholder.typicode.com'
* path 'users'
* method get
* status 200
* match response[0] contains { id: 1, name: 'Leanne Graham', website: 'hildegard.org' }
