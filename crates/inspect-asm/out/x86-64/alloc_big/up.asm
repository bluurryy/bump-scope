inspect_asm::alloc_big::up:
	push rbx
	mov rax, rdi
	mov rcx, qword ptr [rdi]
	mov rdi, qword ptr [rcx]
	dec rdi
	and rdi, -512
	mov r8, rdi
	add r8, 1024
	mov rdx, -1
	cmovae rdx, r8
	cmp rdx, qword ptr [rcx + 8]
	ja .LBB0_0
	mov qword ptr [rcx], rdx
	add rdi, 512
	je .LBB0_0
	mov edx, 512
	pop rbx
	jmp qword ptr [rip + memcpy@GOTPCREL]
.LBB0_0:
	mov rdi, rax
	mov rbx, rsi
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_sized_in_another_chunk
	mov rsi, rbx
	mov rdi, rax
	mov edx, 512
	pop rbx
	jmp qword ptr [rip + memcpy@GOTPCREL]
