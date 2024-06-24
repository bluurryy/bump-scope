inspect_asm::alloc_iter_u32::try_up:
	push rbp
	push r15
	push r14
	push r12
	push rbx
	sub rsp, 32
	test rdx, rdx
	je .LBB0_1
	mov rax, rdx
	shr rax, 61
	je .LBB0_2
.LBB0_0:
	xor eax, eax
	jmp .LBB0_7
.LBB0_1:
	mov eax, 4
	xor edx, edx
	jmp .LBB0_7
.LBB0_2:
	mov rbx, rsi
	lea r15, [4*rdx]
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	mov rsi, qword ptr [rcx + 8]
	add rax, 3
	and rax, -4
	sub rsi, rax
	cmp r15, rsi
	ja .LBB0_8
	lea rsi, [rax + r15]
	mov qword ptr [rcx], rsi
	test rax, rax
	je .LBB0_8
.LBB0_3:
	mov qword ptr [rsp], rax
	mov qword ptr [rsp + 8], 0
	mov qword ptr [rsp + 16], rdx
	mov qword ptr [rsp + 24], rdi
	xor r12d, r12d
	mov r14, rsp
	xor edx, edx
	jmp .LBB0_5
.LBB0_4:
	mov dword ptr [rax + 4*rdx], ebp
	inc rdx
	mov qword ptr [rsp + 8], rdx
	add r12, 4
	cmp r15, r12
	je .LBB0_6
.LBB0_5:
	mov ebp, dword ptr [rbx + r12]
	cmp qword ptr [rsp + 16], rdx
	jne .LBB0_4
	mov rdi, r14
	call bump_scope::bump_vec::BumpVec<T,A,_,_,_>::generic_grow_cold
	test al, al
	jne .LBB0_0
	mov rax, qword ptr [rsp]
	mov rdx, qword ptr [rsp + 8]
	jmp .LBB0_4
.LBB0_6:
	mov rax, qword ptr [rsp]
.LBB0_7:
	add rsp, 32
	pop rbx
	pop r12
	pop r14
	pop r15
	pop rbp
	ret
.LBB0_8:
	mov r14, rdi
	mov rsi, rdx
	mov r12, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_slice_in_another_chunk
	mov rdi, r14
	mov rdx, r12
	test rax, rax
	jne .LBB0_3
	jmp .LBB0_0
