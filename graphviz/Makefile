
# dot -Tpng install.dot > install.png

graphs = run.png install.png select.png

all: $(graphs)

%.png: %.dot
	dot -Tpng $< > $@
