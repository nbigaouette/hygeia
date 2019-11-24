
# dot -Tpng install.dot > install.png

graphs = install.png list.png run.png select.png

all: $(graphs)

%.png: %.dot
	dot -Tpng $< > $@
