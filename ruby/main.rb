require 'webrick'
require 'erb'
require 'open-uri'
require 'rss'

srv = WEBrick::HTTPServer.new(
  :DocumentRoot => './',
  :BindAddress => '',
  :Port => 8080,
  :DirectoryIndex => ['index.html'],
)
srv.config[:MimeTypes]['erb'] = 'text/html'
WEBrick::HTTPServlet::FileHandler.add_handler("erb", WEBrick::HTTPServlet::ERBHandler)
#srv.mount('rss.erb', WEBrick::HTTPServlet::ERBHandler, 'rss.erb')

trap("INT"){ srv.shutdown }
srv.start

